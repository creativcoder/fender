
## Fender

Fender is a helper tool (a scraper) to extract bike data from https://www.canyon.com website
to aid in a data driven bike project.

## Pre-requisites

* Install Rust by running: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
* Install `geckodriver`: https://medium.com/@deepankosingha/how-to-install-geckodriver-on-ubuntu-94b2075b5ad3 and add it to your `$PATH`


## Running

`./scrape <bike_list_url_from_canyon.in site> <folder_to_save_to>` - To start running geckodriver along with `fender` binary

For example: `./scrape https://www.canyon.com/en-in/all-road-bikes/ road-bikes ~/geometry_data`
