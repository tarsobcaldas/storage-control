use chrono::{DateTime, NaiveDate};
use log::info;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Display, Formatter},
};
use ErrorMessage::*;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Hash, PartialOrd)]
pub enum Quality {
    Normal,
    Fragile(usize),
    Oversized(usize),
    OversizedAndFragile(usize, usize),
}

impl Quality {
    pub fn to_string(&self) -> String {
        match self {
            Quality::Normal => "normal".to_string(),
            Quality::Fragile(_) => "fragile".to_string(),
            Quality::Oversized(_) => "oversized".to_string(),
            Quality::OversizedAndFragile(_, _) => "oversized and fragile".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Product {
    pub id: u64,
    pub name: String,
    pub price: u64,
    pub quantity: usize,
    pub quality: Quality,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProductItem {
    pub id: u64,
    pub placement: (usize, usize, usize, usize),
    pub zones_required: usize,
    pub expiry_date: Option<NaiveDate>,
    pub timestamp: DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductList {
    pub products: HashMap<u64, Product>,
}

impl Display for Product {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let price = format_price(self.price);
        write!(
            f,
            "Product: {}\n ID: {}, Price: {}, Quantity: {}",
            self.name, self.id, price, self.quantity,
        )
    }
}
impl Display for ProductItem {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let date = match self.expiry_date {
            Some(date) => date.to_string(),
            None => "N/A".to_string(),
        };
        write!(
            f,
            "ID: {}, Location: {:?}, Expiry Date: {}",
            self.id, self.placement, date
        )
    }
}

#[derive(Debug)]
pub struct ProductError {
    pub level: String,
    pub message: String,
}

impl Display for ProductError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} Error: {}", self.level, self.message)
    }
}

impl Error for ProductError {}

impl ProductError {
    pub fn boxed(level: &str, message: String) -> Box<dyn Error> {
        Box::new(ProductError {
            level: level.to_string(),
            message: message.to_string(),
        })
    }

    pub fn product(message: String) -> Box<dyn Error> {
        ProductError::boxed("Product", message)
    }

    pub fn list(message: String) -> Box<dyn Error> {
        ProductError::boxed("List", message)
    }

    pub fn item(message: String) -> Box<dyn Error> {
        ProductError::boxed("Item", message)
    }

    pub fn message(error: ErrorMessage, details: Option<String>) -> String {
        if let Some(details) = details {
            format!("{}: {}", error, details)
        } else {
            format!("{}", error)
        }
    }
}

#[derive(Debug)]
pub enum ErrorMessage {
    NotEnoughQuantity,
    ProductNotFound,
    NameExists,
    InvalidInput,
    LevelTooHigh,
    FragileObjectWithoutExpiration,
}

impl ErrorMessage {
    pub fn as_str(&self) -> &str {
        match self {
            NotEnoughQuantity => "Not enough quantity",
            ProductNotFound => "Product not listed",
            NameExists => "Product with this name already exists",
            InvalidInput => "Invalid input",
            LevelTooHigh => "Level too high",
            FragileObjectWithoutExpiration => "Fragile object without expiration",
        }
    }
}

impl PartialEq for ErrorMessage {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl From<ErrorMessage> for String {
    fn from(val: ErrorMessage) -> Self {
        val.as_str().to_string()
    }
}

impl Display for ErrorMessage {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

fn format_price(price: u64) -> String {
    let numeral = price / 100;
    let decimal = price % 100;

    format!("${}.{}", numeral, format_args!("{:02}", decimal))
}

#[allow(dead_code)]
impl Product {
    pub fn new(name: &str, price: u64, quantity: usize, quality: Quality) -> Self {
        Product {
            id: generate_id(),
            name: name.to_string(),
            price,
            quantity,
            quality,
        }
    }

    pub fn add_quantity(&mut self, quantity: usize) {
        self.quantity += quantity;
    }

    pub fn remove_quantity(&mut self, quantity: usize) -> Result<(), Box<dyn Error>> {
        match self.quantity >= quantity {
            true => {
                self.quantity -= quantity;
                Ok(())
            }
            false => {
                let message = ProductError::message(NotEnoughQuantity, None);
                Err(ProductError::product(message))
            }
        }
    }

    pub fn set_price(&mut self, price: u64) {
        self.price = price;
    }

    pub fn set_quality(&mut self, quality: Quality) {
        self.quality = quality;
    }

    pub fn max_level(&self) -> Option<usize> {
        match self.quality {
            Quality::Fragile(maxlevel) => Some(maxlevel),
            Quality::OversizedAndFragile(_, maxlevel) => Some(maxlevel),
            _ => None,
        }
    }

    pub fn print_price(&self) {
        println!("Price: {}", format_price(self.price));
    }
}

#[allow(dead_code)]
impl ProductItem {
    pub fn new(
        id: u64,
        list: &mut ProductList,
        placement: (usize, usize, usize, usize),
        expiry_date: Option<NaiveDate>,
    ) -> Result<Self, Box<dyn Error>> {
        use Quality::*;
        match list.product_mut(id) {
            Some(product) => match product.quality {
                Fragile(maxlevel) => {
                    if expiry_date.is_none() {
                        let message = ProductError::message(FragileObjectWithoutExpiration, None);
                        return Err(ProductError::item(message));
                    }
                    if placement.2 > maxlevel {
                        let message = ProductError::message(LevelTooHigh, None);
                        return Err(ProductError::item(message));
                    }
                    product.add_quantity(1);
                    Ok(ProductItem {
                        id,
                        zones_required: 1,
                        placement,
                        expiry_date,
                        timestamp: chrono::Utc::now(),
                    })
                }
                Oversized(zones_required) => {
                    product.add_quantity(1);
                    Ok(ProductItem {
                        id,
                        placement,
                        zones_required,
                        expiry_date,
                        timestamp: chrono::Utc::now(),
                    })
                }
                OversizedAndFragile(zones_required, maxlevel) => {
                    if expiry_date.is_none() {
                        let message = ProductError::message(FragileObjectWithoutExpiration, None);
                        return Err(ProductError::item(message));
                    }
                    if placement.2 > maxlevel {
                        let message = ProductError::message(LevelTooHigh, None);
                        return Err(ProductError::item(message));
                    }
                    product.add_quantity(1);
                    Ok(ProductItem {
                        id,
                        placement,
                        zones_required,
                        expiry_date,
                        timestamp: chrono::Utc::now(),
                    })
                }
                _ => {
                    product.add_quantity(1);
                    Ok(ProductItem {
                        id,
                        placement,
                        zones_required: 1,
                        expiry_date,
                        timestamp: chrono::Utc::now(),
                    })
                }
            },
            None => {
                let message = ProductError::message(ProductNotFound, None);
                Err(ProductError::item(message))
            },
        }
    }

    pub fn place(&mut self, file: usize, shelf: usize, level: usize, zone: usize) {
        self.placement = (file, shelf, level, zone);
    }

    pub fn set_expiration(&mut self, expiry_date: Option<NaiveDate>) {
        self.expiry_date = expiry_date;
    }
}

#[allow(dead_code)]
impl ProductList {
    pub fn new() -> Self {
        ProductList {
            products: HashMap::new(),
        }
    }

    pub fn with(products: HashMap<u64, Product>) -> Self {
        ProductList { products }
    }

    pub fn add(&mut self, mut product: Product) -> Result<(), Box<dyn Error>> {
        loop {
            if self.products.contains_key(&product.id) {
                product.id = generate_id();
            } else {
                break;
            }
        }
        if self.products.values().any(|p| p.name == product.name) {
            let message = ProductError::message(NameExists, Some(format!("- {}", product.name)));
            return Err(ProductError::list(message));
        }
        info!("Product {} added", product.id);
        self.products.insert(product.id, product);
        Ok(())
    }

    pub fn remove_by_id(&mut self, id: u64) -> Result<(), Box<dyn Error>> {
        if self.products.remove(&id).is_some() {
            info!("Product {} removed", id);
            Ok(())
        } else {
            let message = ProductError::message(ProductNotFound, Some(format!("- {}", id)));
            Err(ProductError::list(message))
        }
    }

    pub fn remove_by_name(&mut self, name: &str) -> Result<(), Box<dyn Error>> {
        let id = match self.products.values().find(|p| p.name == name) {
            Some(product) => product.id,
            None => {
                let message = ProductError::message(ProductNotFound, Some(format!("- {}", name)));
                return Err(ProductError::list(message));
            },
        };
        match self.remove_by_id(id) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub fn product(&self, id: u64) -> Option<&Product> {
        self.products.get(&id)
    }

    pub fn product_mut(&mut self, id: u64) -> Option<&mut Product> {
        self.products.get_mut(&id)
    }

    pub fn step_qty(&mut self, id: u64, quantity: isize) -> Result<(), Box<dyn Error>> {
        match self.product_mut(id) {
            Some(product) => {
                if quantity < 0 && product.quantity < quantity.unsigned_abs() {
                    let message = ProductError::message(NotEnoughQuantity, None);
                    return Err(ProductError::list(message));
                }
                product.quantity = (product.quantity as isize + quantity) as usize;
                Ok(())
            }
            None => {
                let message = ProductError::message(ProductNotFound, Some(format!("- {}", id)));
                Err(ProductError::list(message))
            }
        }
    }

    pub fn empty_qty(&mut self, id: u64) -> Result<(), Box<dyn Error>> {
        match self.product_mut(id) {
            Some(product) => {
                product.quantity = 0;
                Ok(())
            }
            None => {
                let message = ProductError::message(ProductNotFound, Some(format!("- {}", id)));
                Err(ProductError::list(message))
            }
        }
    }

    pub fn id_from_name(&self, name: &str) -> Option<u64> {
        self.products
            .values()
            .find(|p| p.name == name)
            .map(|product| product.id)
    }

    pub fn list(&self) {
        for product in self.products.values() {
            println!("{}", product);
        }
    }

    pub fn filter_by_quality(&self, quality: String) -> Vec<&Product> {
        self.products
            .values()
            .filter(|product| product.quality.to_string() == quality)
            .collect()
    }

    pub fn filter_by_max_price(&self, price: u64) -> Vec<&Product> {
        self.products
            .values()
            .filter(|product| product.price <= price)
            .collect()
    }

    pub fn filter_by_min_price(&self, price: u64) -> Vec<&Product> {
        self.products
            .values()
            .filter(|product| product.price >= price)
            .collect()
    }

    pub fn search_by_name(&self, string: &str) -> Vec<&Product> {
        let string = string.to_lowercase();
        let words: Vec<&str> = string.split_whitespace().collect();
        self.products
            .values()
            .filter(|product| {
                words.iter().all(|word| product.name.to_lowercase().contains(word))
            })
            .collect()
    }
}

impl Default for ProductList {
    fn default() -> Self {
        let mut products = ProductList::new();
        products
            .add(Product::new("Apple", 100, 0, Quality::Normal))
            .unwrap();
        products
            .add(Product::new("Banana", 50, 0, Quality::Fragile(3)))
            .unwrap();
        products
            .add(Product::new("Watermelon", 75, 0, Quality::Oversized(3)))
            .unwrap();
        products
    }
}

fn generate_id() -> u64 {
    let mut random = rand::thread_rng();
    let id: u64 = random.gen_range(100000..999999);
    id
}
