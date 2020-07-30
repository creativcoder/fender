
mod logger;
mod cli;

use std::process::Command;
use fantoccini::{Client, Locator};
use tokio;
use std::time::Duration;
use select::{document::Document, predicate::{Predicate, Attr, Class, Name}};
use csv::Writer as CsvWriter;
use rand::Rng;

async fn skip_newsletter() {
    let newsletter_xpath = "/html/body/div[1]/footer/div/div[3]/div[3]";
}

async fn crawl_by_type(url: &str, c: &mut Client, category: &str) -> Result<(), fantoccini::error::CmdError> {
    c.goto(url).await?;
    let cookies_btn = c.find(Locator::Id("js-data-privacy-save-button")).await?;
    cookies_btn.click().await?;

    let mut rng = rand::thread_rng();
    let jitter: u64 = rng.gen_range(1000, 5000);

    tokio::time::delay_for(Duration::from_millis(jitter)).await;

    let mut idx: u32 = 0;

    while let Ok(view_more_btn) = c.find(Locator::Css("button.button--secondary:nth-child(1)")).await {
        idx += 1;

        let _click_res = view_more_btn.click().await?;
        println!("Failed clicking, retrying");
        let jitter: u64 = rng.gen_range(5000, 10000);
        tokio::time::delay_for(Duration::from_millis(jitter)).await;

        let jitter: u64 = rng.gen_range(5000, 10000);
        
        tokio::time::delay_for(Duration::from_millis(jitter)).await;
    }

    println!("Clicked {} times", idx);

    let current_page_html_source = c.source().await.unwrap();
    let cursor = std::io::Cursor::new(current_page_html_source);
    let doc = Document::from_read(cursor).unwrap();
    // item is each bike on the list of bikes shown on aerod_url
    for item in doc.find(Attr("class", "productTile__link")) {
        // For each bike on this list we have certain properties on attrs method
        let bike_name = item.attrs().find(|(a, b)| {
            *a == "title"
        }).unwrap().1.replace(" ", "_").to_lowercase();

        let bike_url = item.attrs().find(|(a, b)| {
            *a == "href"
        }).unwrap().1;

        tokio::time::delay_for(Duration::from_millis(2000u64)).await;
        c.goto(bike_url).await.unwrap();
        let specific_bike_html_page = c.source().await.unwrap();
        extract_geometry_from_bike_page(&specific_bike_html_page, &bike_name, category).await;
    }

    Ok(())
}

async fn crawl_bikes_by_type(bike_type_urls: Vec<(&'static str, &'static str)>) -> Result<(), fantoccini::error::CmdError> {
    let mut c = Client::new("http://localhost:4444").await.expect("failed to connect to WebDriver");
    for (category, types_url) in bike_type_urls {
        crawl_by_type(types_url, &mut c, category).await?;
    }

    Ok(())
}

async fn extract_geometry_from_bike_page(src: &str, bike_name: &str, category: &str) {
    let cursor = std::io::Cursor::new(src);
    let doc = Document::from_read(cursor).unwrap();
    let directory_structure = format!("./{}", category);
    std::fs::create_dir_all(directory_structure).unwrap();

    let mut output = CsvWriter::from_path(format!("./{}/{}.csv", category, bike_name)).unwrap();
    // Access geometry table for this bike
    for node in doc.find(Attr("class", "geometryTable__table").descendant(Name("thead"))) {
        let split = node.text();
        let split = split.split("\n");
        // This is the geometry table head which has details on the sizes
        let mut theads = vec!["attributes"];
        for line in split {
            let line = line.trim();
            if !line.is_empty() {
                theads.push(line);
            }
        }

        info!("Writing theads: {:?}", theads);
        output.write_record(&theads).unwrap();
    }

    // Process all the rows with values in geometry table
    for node in doc.find(Attr("class", "geometryTable__table").descendant(Name("tbody"))) {
        // A single row in the geometry table page
        for table_row in node.find(Class("geometryTable__dataRow")) {
            let mut table_row_vec = vec![];
            let mut title = table_row.find(Class("geometryTable__titleInner"));
            let child = title.next().unwrap();
            // Table row head
            let row_head = child.text();
            let row_head = row_head.trim();
            table_row_vec.push(row_head.to_string());

            // Table row items
            info!("Writing data points for {}", bike_name);
            for td in table_row.find(Class("geometryTable__sizeData")) {
                if let Some(data) = td.children().next() {
                    let table_row_value = data.text().trim().to_string();
                    table_row_vec.push(table_row_value);
                } else {
                    table_row_vec.push("".to_string());
                }
            }

            output.write_record(&table_row_vec).unwrap();
        }
    }
}

// newsletter xpath: /html/body/div[1]/footer/div/div[3]/div[3]

async fn crawl_specific_bike(bike_brand_url: &str, brand: &str) -> Result<(), fantoccini::error::CmdError> {
    let mut c = Client::new("http://localhost:4444").await.expect("failed to connect to WebDriver");
    c.goto(bike_brand_url).await?;
    // wait and click accept cookies
    tokio::time::delay_for(Duration::from_millis(3000)).await;
    let cookies_btn = c.find(Locator::Id("js-data-privacy-save-button")).await?;
    cookies_btn.click().await?;
    let current_page_html_source = c.source().await.unwrap();
    let cursor = std::io::Cursor::new(current_page_html_source);
    let doc = Document::from_read(cursor).unwrap();
    // item is each bike on the list of bikes shown on aerod_url
    for item in doc.find(Attr("class", "productTile__link")) {
        // For each bike on this list we have certain properties on attrs method
        let bike_name = item.attrs().find(|(a, b)| {
            *a == "title"
        }).unwrap().1.replace(" ", "_").to_lowercase();

        let bike_url = item.attrs().find(|(a, b)| {
            *a == "href"
        }).unwrap().1;

        tokio::time::delay_for(Duration::from_millis(2000u64)).await;
        c.goto(bike_url).await.unwrap();
        let specific_bike_html_page = c.source().await.unwrap();
        extract_geometry_from_bike_page(&specific_bike_html_page, &bike_name, brand).await;
    }

    Ok(())
}

fn stop_geckodriver() {
    let _ = Command::new("pkill")
    .args(&["geckodriver"])
    .output()
    .expect("failed to kill geckodriver");
}

#[tokio::main]
async fn main() -> Result<(), fantoccini::error::CmdError> {
    ctrlc::set_handler(move || {
        println!("Killing geckodriver");
        stop_geckodriver();
    }).expect("Failed handling Ctrl+C");

    let _ = Command::new("geckodriver")
            .args(&["&"])
            .output()
            .expect("failed to execute process");
    let args: cli::FenderArgs = argh::from_env();
    let specific_bike_url = args.bike_url;
    let bike_type = args.bike_type;

    crawl_specific_bike(&specific_bike_url, &bike_type).await.expect("Failed to crawl bike");

    // // Run crawler by bike type
    // let bike_types_urls = vec![
    //     ("road", "https://www.canyon.com/en-in/all-road-bikes/"),
    //     ("mountain", "https://www.canyon.com/en-in/all-mountain-bikes/"),
    //     ("city-hybrid", "https://www.canyon.com/en-in/all-city-hybrid-bikes/"),
    //     ("woman-road", "https://www.canyon.com/en-in/all-women-bikes/?hideSelectedFilters=true&map_prefn1=pc_welt&map_prefv1=Road"),
    //     ("woman-mountain", "https://www.canyon.com/en-in/all-women-bikes/?hideSelectedFilters=true&map_prefn1=pc_welt&map_prefv1=Mountain%20Bike")
    // ];

    // crawl_bikes_by_type(bike_types_urls).await.unwrap();
    stop_geckodriver();
    Ok(())
}
