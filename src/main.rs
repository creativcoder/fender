mod cli;
mod logger;

use csv::Writer as CsvWriter;
use fantoccini::{Client, Locator};
use rand::Rng;
use select::{
    document::Document,
    predicate::{Attr, Class, Name, Predicate},
};
use std::process::Command;
use std::time::Duration;
use tokio;

async fn skip_newsletter(c: &mut Client) {
    let newsletter_close_btn = c.find(Locator::XPath("/html/body/div[1]/footer/div/div[3]/div[3]/div/div/div/div/div[1]/div/button/svg")).await;
    if let Ok(btn) = newsletter_close_btn {
        dbg!("Yo");
        btn.click().await.expect("Failed clicking on the newsletter close button");
    }
}

async fn crawl_by_type(
    url: &str,
    c: &mut Client,
    category: &str,
    output_location: &str
) -> Result<(), fantoccini::error::CmdError> {
    c.goto(url).await?;
    let cookies_btn = c.find(Locator::Id("js-data-privacy-save-button")).await?;
    cookies_btn.click().await?;

    let mut rng = rand::thread_rng();
    let jitter: u64 = rng.gen_range(1000, 5000);

    tokio::time::delay_for(Duration::from_millis(jitter)).await;

    let mut idx: u32 = 0;

    while let Ok(view_more_btn) = c
        .find(Locator::Css("button.button--secondary:nth-child(1)"))
        .await
    {
        idx += 1;

        let _click_res = view_more_btn.click().await?;
        println!("Failed clicking, retrying");
        let jitter: u64 = rng.gen_range(5000, 10000);
        tokio::time::delay_for(Duration::from_millis(jitter)).await;

        let jitter: u64 = rng.gen_range(5000, 10000);

        tokio::time::delay_for(Duration::from_millis(jitter)).await;
    }

    let current_page_html_source = c.source().await.unwrap();
    let cursor = std::io::Cursor::new(current_page_html_source);
    let doc = Document::from_read(cursor).unwrap();
    // item is each bike on the list of bikes shown on aerod_url
    for item in doc.find(Attr("class", "productTile__link")) {
        // For each bike on this list we have certain properties on attrs method
        let bike_name = item
            .attrs()
            .find(|(a, _)| *a == "title")
            .unwrap()
            .1
            .replace(" ", "_")
            .to_lowercase();

        let bike_url = item.attrs().find(|(a, _)| *a == "href").unwrap().1;

        tokio::time::delay_for(Duration::from_millis(2000u64)).await;
        c.goto(bike_url).await.unwrap();
        let specific_bike_html_page = c.source().await.unwrap();
        extract_geometry_from_bike_page(&specific_bike_html_page, &bike_name, category, output_location).await;
    }

    Ok(())
}

async fn crawl_bikes_by_type(
    bike_type_urls: Vec<(&'static str, &'static str)>,
    output_location: &str
) -> Result<(), fantoccini::error::CmdError> {
    let mut c = Client::new("http://localhost:4444")
        .await
        .expect("failed to connect to WebDriver");
    for (category, types_url) in bike_type_urls {
        crawl_by_type(types_url, &mut c, category, output_location).await?;
    }

    Ok(())
}

async fn extract_geometry_from_bike_page(src: &str, bike_name: &str, category: &str, output_location: &str) {
    let cursor = std::io::Cursor::new(src);
    let doc = Document::from_read(cursor).unwrap();

    let directory_structure = format!("{}/{}", output_location, category);
    info!("Created directory at {}",directory_structure);
    std::fs::create_dir_all(&directory_structure).unwrap();

    let mut output = CsvWriter::from_path(format!("{}/{}.csv", directory_structure, bike_name)).unwrap();
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

async fn crawl_specific_bike(
    bike_brand_url: &str,
    _type: &str,
    output_location: &str,
) -> Result<(), fantoccini::error::CmdError> {
    info!(
        "Launching scraper. Output will be stored in {}",
        output_location
    );
    let mut c = Client::new("http://localhost:4444")
        .await
        .expect("failed to connect to WebDriver");
    c.goto(bike_brand_url).await?;
    // wait and click accept cookies
    tokio::time::delay_for(Duration::from_millis(3000)).await;
    let cookies_btn = c.find(Locator::Id("js-data-privacy-save-button")).await?;
    cookies_btn.click().await?;
    let current_page_html_source = c.source().await?;
    let cursor = std::io::Cursor::new(current_page_html_source);
    let doc = Document::from_read(cursor)?;
    // item is each bike on the list of bikes shown on aerod_url
    for item in doc.find(Attr("class", "productTile__link")) {
        // For each bike on this list we have certain properties on attrs method
        let bike_name = item
            .attrs()
            .find(|(a, _)| *a == "title")
            .unwrap()
            .1
            .replace(" ", "_")
            .to_lowercase();

        let bike_url = item.attrs().find(|(a, _)| *a == "href").unwrap().1;

        tokio::time::delay_for(Duration::from_millis(2000u64)).await;
        c.goto(bike_url).await?;
        info!("Navigating to {}", bike_url);
        let specific_bike_html_page = c.source().await?;
        skip_newsletter(&mut c).await;
        extract_geometry_from_bike_page(&specific_bike_html_page, &bike_name, _type, output_location).await;
    }

    Ok(())
}

fn stop_geckodriver() {
    let _ = Command::new("pkill")
        .args(&["geckodriver"])
        .output()
        .expect("failed to kill geckodriver");
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: cli::FenderArgs = argh::from_env();
    let specific_bike_url = args.bike_url;
    let base_url = url::Url::parse(&specific_bike_url).unwrap();
    let brand = base_url.host_str().unwrap().split(".").skip(1).next().expect("Failed to extract bike brand from url");
    let bike_type = args.bike_type;
    let output_location = format!("{}/{}", args.output, brand);

    crawl_specific_bike(&specific_bike_url, &bike_type, &output_location)
        .await
        .expect(&format!("Failed to crawl bike at {}", specific_bike_url));
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), fantoccini::error::CmdError> {
    ctrlc::set_handler(move || {
        println!("Killing geckodriver");
        stop_geckodriver();
    })
    .expect("Failed handling Ctrl+C");

    if let Err(_) = run().await {
        stop_geckodriver();
    }

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
    info!("Done!");
    Ok(())
}
