# Energy Strategy Scrapers Documentation

This document provides technical details on how to use the `ve_energy_scrapers` crate. It is intended for AI assistants and developers integrating this library.

## Overview

`ve_energy_scrapers` is a Rust library for scraping energy market data from various sources (currently APG and ENTSO-E). It defines a common `Scraper` trait and provides implementations for specific data providers.

**Recent Update (Jan 2026):** The scrapers have been refactored to be **date-parameterized**. Instead of hardcoding date logic within the scrapers (e.g., fetching "today" and "tomorrow"), the calling code now specifies the exact date range to scrape. This provides greater flexibility for fetching historical data, custom date ranges, or implementing custom time offset logic.

## Installation

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
ve_energy_scrapers = { path = "../Scrapers" } # Or git/crates.io reference
anyhow = "1.0"
async-trait = "0.1"
chrono = "0.4"
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
```

## Core Components

### `Scraper` Trait

The core interface is the `Scraper` trait, defined in `src/scraper.rs`.

```rust
#[async_trait]
pub trait Scraper: Send + Sync {
    fn get_config(&self) -> &StrategyInformationScraperConfig;
    async fn scrape_data(&self, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<Vec<ScraperData>>;
}
```

**Key Change:** The `scrape_data` method now requires `start_date` and `end_date` parameters, allowing the caller to specify exactly which time period to scrape.

### Data Models

The data returned by scrapers is structured using the `ScraperData` struct defined in `src/models/scraper_data.rs`.

```rust
pub struct ScraperData {
    pub delivery_from: DateTime<Utc>,
    pub delivery_to: DateTime<Utc>,
    pub payload: ScraperPayload,
}

pub enum ScraperPayload {
    Values(HashMap<String, f64>),
    Bids(Vec<Bid>),
}
```

**Bid Structure**
The `Bid` struct is designed to be `Copy`-compliant for efficient async handling.

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Bid {
    pub price: Option<f64>,
    pub volume: Option<f64>,
    pub bid_type: BidType,
    pub direction: BidDirection,
    pub rank: i32,
}

pub enum BidType {
    SRE, // Secondary Regulation Energy
    TRE, // Tertiary Regulation Energy
}

pub enum BidDirection {
    POS, // Positive
    NEG, // Negative
}
```

### `StrategyInformationScraperConfig`

Configuration struct used to initialize scrapers. It supports dynamic key-value pairs via a flattened HashMap.

```rust
pub struct StrategyInformationScraperConfig {
    pub name: String,
    pub workers: u32,
    pub task_generator_delay_ms: u32,
    pub values: HashMap<String, Value>, // Dynamic configuration values
}
```

## Implementations

### 1. APG Information Scraper

Scrapes data from the Austrian Power Grid (APG).

**Module**: `ve_energy_scrapers::apg_information_scraper`
**Struct**: `APGInformationScraper`

**Initialization**:
```rust
use ve_energy_scrapers::apg_information_scraper::APGInformationScraper;

let scraper = APGInformationScraper::new(config)?;
```

**Required Config Keys**:
- `url`: Base URL for the APG API.
- `url_template`: Template string for the API endpoint (e.g., `DRZ/Data/English/PT15M/{from}/{to}?p_drzMode=OperationalOrSettlement`).
- `value_column` (or `value_columns`): The internal name(s) of the column(s) to extract from the response.

**Optional Config Keys**:
- `is_balancing_bids`: Boolean flag to indicate if the scraper should parse balancing bids (default: false).

**Note:** The `time_offset_minutes` config option has been removed. The calling code should now adjust the date range as needed before passing it to `scrape_data()`.

### 2. ENTSO-E Information Scraper

Scrapes data from the ENTSO-E Transparency Platform.

**Module**: `ve_energy_scrapers::entsoe_information_scraper`
**Struct**: `EntsoeInformationScraper`

**Initialization**:
```rust
use ve_energy_scrapers::entsoe_information_scraper::EntsoeInformationScraper;

let scraper = EntsoeInformationScraper::new(config)?;
```

**Required Config Keys**:
- `url`: Base URL for the ENTSO-E API.
- `token`: Security token for authentication.

## Usage Example

Here's how to use the refactored scrapers with custom date ranges:

```rust
use chrono::{Duration, Utc};
use ve_energy_scrapers::scraper::Scraper;
use ve_energy_scrapers::entsoe_information_scraper::EntsoeInformationScraper;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create scraper with config
    let scraper = EntsoeInformationScraper::new(config)?;
    
    // Define custom date range
    let now = Utc::now();
    let start_date = now - Duration::days(1);  // Yesterday
    let end_date = now + Duration::days(1);    // Tomorrow
    
    // Scrape data for the specified date range
    let data = scraper.scrape_data(start_date, end_date).await?;
    
    println!("Fetched {} data points", data.len());
    
    // Fetch historical data for a specific date
    let historical_start = Utc::now() - Duration::days(30);
    let historical_end = historical_start + Duration::days(1);
    let historical_data = scraper.scrape_data(historical_start, historical_end).await?;
    
    Ok(())
}
```

See `examples/usage_example.rs` for a complete working example.

### Migration Guide

If you're migrating from the old API where `scrape_data()` took no parameters:

**Old Code:**
```rust
let scraper = APGInformationScraper::new(config)?;
let data = scraper.scrape_data().await?;
```

**New Code:**
```rust
use chrono::{Duration, Utc};

let scraper = APGInformationScraper::new(config)?;

// Define the date range you want to scrape
let now = Utc::now();
let start_date = now - Duration::minutes(30);  // Adjust as needed
let end_date = start_date + Duration::days(2);

let data = scraper.scrape_data(start_date, end_date).await?;
```

**What Changed:**
1. `scrape_data()` now requires two parameters: `start_date: DateTime<Utc>` and `end_date: DateTime<Utc>`
2. The `time_offset_minutes` configuration option has been removed - apply time offsets to your dates before calling the scraper
3. Both scrapers now fetch data for the exact date range you specify, rather than automatically fetching "today" and "tomorrow"

## Complete Example

```rust
use std::collections::HashMap;
use serde_json::Value;
use chrono::{Duration, Utc};
use ve_energy_scrapers::models::strategy_information_scraper_config::StrategyInformationScraperConfig;
use ve_energy_scrapers::apg_information_scraper::APGInformationScraper;
use ve_energy_scrapers::scraper::Scraper;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Create Configuration
    let mut values = HashMap::new();
    values.insert("url".to_string(), Value::String("https://transparency.apg.at/api/v1".to_string()));
    values.insert("url_template".to_string(), Value::String("ATC/Data/English/PT15M/{from}/{to}?p_border=AT<>DE".to_string()));
    values.insert("value_columns".to_string(), Value::Array(vec![
        Value::String("OfferedCapacityDEtoAT".to_string()),
        Value::String("OfferedCapacityATtoDE".to_string()),
    ]));

    let config = StrategyInformationScraperConfig {
        name: "APG_Scraper_Test".to_string(),
        workers: 1,
        task_generator_delay_ms: 1000,
        values,
    };

    // 2. Initialize Scraper
    let scraper = APGInformationScraper::new(config)?;

    // 3. Define date range
    let now = Utc::now();
    let start_date = now - Duration::days(1);
    let end_date = now + Duration::days(1);

    // 4. Scrape Data
    let data = scraper.scrape_data(start_date, end_date).await?;

    println!("Fetched {} data points", data.len());

    Ok(())
}
```
