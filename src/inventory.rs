use crate::{
    product::{Product, ProductList, Quality},
    warehouse::Warehouse,
};
use chrono::NaiveDate;
use log::info;
use serde::{Deserialize, Serialize};
use serde_json;
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    fs::File,
    io::{self, BufReader, Write},
};
use ErrorMessage::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct Storage {
    pub name: String,
    pub list: ProductList,
    pub file_path: String,
    pub warehouse: Warehouse,
}

#[derive(Debug)]
pub enum ErrorMessage {
    ProductNotFound,
    HasStock,
}

#[derive(Debug)]
struct StorageError {
    message: String,
}

impl Display for ErrorMessage {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let message = self.as_str();
        write!(f, "Storage error: {}", message)
    }
}

impl ErrorMessage {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            ProductNotFound => "Product Not Found",
            HasStock => "Product has stock",
        }
    }
}

impl Display for StorageError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for StorageError {}

impl StorageError {
    pub fn boxed(message: String) -> Box<dyn Error> {
        Box::new(StorageError {
            message: message.to_string(),
        })
    }

    pub fn list(message: ErrorMessage) -> Box<dyn Error> {
        StorageError::boxed(format!("List error: {}", message))
    }
}

#[allow(dead_code)]
impl Storage {
    pub fn new(name: String, file_path: Option<String>) -> Self {
        let default_path = format!("./storage_{}.json", name);
        Storage {
            name,
            list: ProductList::new(),
            warehouse: Warehouse::new(),
            file_path: file_path.unwrap_or(default_path),
        }
    }

    pub fn save(&self) -> io::Result<()> {
        match File::create(&self.file_path) {
            Ok(mut file) => match serde_json::to_string_pretty(self) {
                Ok(json) => file.write_all(json.as_bytes()),
                Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
            },
            Err(e) => Err(e),
        }
    }

    pub fn save_as(&self, file_path: &str) -> io::Result<()> {
        match File::create(file_path) {
            Ok(mut file) => match serde_json::to_string_pretty(self) {
                Ok(json) => file.write_all(json.as_bytes()),
                Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
            },
            Err(e) => Err(e),
        }
    }

    pub fn load<'a>(
        file_path: &str,
        storage: &'a mut Storage,
    ) -> Result<&'a mut Storage, Box<dyn Error>> {
        let path = file_path;
        match File::open(path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                match serde_json::from_reader::<BufReader<File>, Storage>(reader) {
                    Ok(new_storage) => {
                        storage.name = new_storage.name;
                        storage.list = new_storage.list;
                        storage.warehouse = new_storage.warehouse;
                        storage.file_path = new_storage.file_path;

                        Ok(storage)
                    }
                    Err(e) => Err(Box::new(e)),
                }
            }
            Err(e) => Err(Box::new(e)),
        }
    }

    pub fn check_capacity(&self) -> usize {
        self.warehouse.check_capacity()
    }

    pub fn check_available_space(&self) -> usize {
        self.warehouse.available_space
    }

    pub fn list_products(&self) {
        for product in self.list.products.values() {
            println!("{}", product);
        }
    }

    pub fn search_product_name(&self, name: &str) {
        self.list.search_by_name(name);
    }

    pub fn list_items(&self) {
        self.warehouse.print_items();
    }

    pub fn list_items_with_id(&self, id: u64) {
        self.warehouse.print_items_with_id(id);
    }

    pub fn list_items_with_name(&self, name: &str) {
        self.warehouse.print_items_with_name(name, &self.list);
    }

    pub fn list_expiring_items(&self, days: u64) {
        let date = chrono::Local::now().naive_local().date();
        let expiring_date = date + chrono::Duration::days(days as i64);
        self.warehouse.print_expiring_items(&self.list, expiring_date);
    }

    pub fn list_expiring_with_id(&self, id: u64, days: u64) {
        let date = chrono::Local::now().naive_local().date();
        let expiring_date = date + chrono::Duration::days(days as i64);
        self.warehouse.print_expiring_with_id(id, &self.list, expiring_date);
    }

    pub fn list_expiring_with_name(&self, name: &str, days: u64) {
        let date = chrono::Local::now().naive_local().date();
        let expiring_date = date + chrono::Duration::days(days as i64);
        self.warehouse
            .print_expiring_with_name(name, &self.list, expiring_date);
    }

    pub fn list_expired_items(&self) {
        let date = chrono::Local::now().naive_local().date();
        self.warehouse.print_expired_items(&self.list, date);
    }

    pub fn list_expired_with_id(&self, id: u64) {
        let date = chrono::Local::now().naive_local().date();
        self.warehouse.print_expired_with_id(id, &self.list, date);
    }

    pub fn list_expired_with_name(&self, name: &str) {
        let date = chrono::Local::now().naive_local().date();
        self.warehouse.print_expired_with_name(name, &self.list, date);
    }

    pub fn list_with_max_price(&self, price: u64) {
        self.list.filter_by_max_price(price).iter().for_each(|product| {
            println!("{}", product);
        });
    }

    pub fn list_with_min_price(&self, price: u64) {
        self.list.filter_by_min_price(price).iter().for_each(|product| {
            println!("{}", product);
        });
    }

    pub fn list_with_quality(&self, quality: String) {
        self.list.filter_by_quality(quality).iter().for_each(|product| {
            println!("{}", product);
        });
    }

    pub fn new_product(
        &mut self,
        name: String,
        price: u64,
        quality: Quality,
    ) -> Result<(), Box<dyn Error>> {
        let product = Product::new(&name, price, 0, quality);
        match self.list.add(product) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub fn find_product_id(&self, name: &str) -> Option<u64> {
        self.list.id_from_name(name)
    }

    pub fn find_product_name(&self, id: u64) -> Option<&str> {
        match self.list.products.get(&id) {
            Some(product) => Some(product.name.as_str()),
            None => None,
        }
    }

    pub fn delete_product_by_id(&mut self, id: u64) -> Result<(), Box<dyn Error>> {
        if let Some(product) = self.list.product(id) {
            if product.quantity > 0 {
                Err(StorageError::list(HasStock))
            } else {
                self.list.remove_by_id(id)?;
                info!("Product {} removed", id);
                Ok(())
            }
        } else {
            Err(StorageError::list(ProductNotFound))
        }
    }

    pub fn delete_product_by_name(&mut self, name: &str) -> Result<(), Box<dyn Error>> {
        match self.find_product_id(name) {
            Some(id) => match self.delete_product_by_id(id) {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            },
            None => Err(StorageError::list(ProductNotFound)),
        }
    }

    pub fn restock_product(
        &mut self,
        id: u64,
        quantity: usize,
        expiry_date: Option<NaiveDate>,
    ) -> Result<(), Box<dyn Error>> {
        match self
            .warehouse
            .independent_restock(id, quantity, &mut self.list, expiry_date)
        {
            Ok(_) => self.list.step_qty(id, quantity as isize),
            Err(e) => Err(e),
        }
    }

    pub fn restock_by_name(
        &mut self,
        name: &str,
        quantity: usize,
        expiry_date: Option<NaiveDate>,
    ) -> Result<(), Box<dyn Error>> {
        let step = quantity as isize;
        match self.find_product_id(name) {
            Some(id) => match self.restock_product(id, quantity, expiry_date) {
                Ok(_) => self.list.step_qty(id, -step),
                Err(e) => Err(e),
            },
            None => Err(StorageError::list(ProductNotFound)),
        }
    }

    pub fn change_price(&mut self, id: u64, price: u64) -> Result<(), Box<dyn Error>> {
        let current_price = self.list.products.get(&id).unwrap().price;
        if let Some(product) = self.list.products.get_mut(&id) {
            product.set_price(price);
            println!(
                "Price for product {} changed from {} to {}",
                id, current_price, price
            );
            Ok(())
        } else {
            Err(StorageError::list(ProductNotFound))
        }
    }

    pub fn change_price_by_name(&mut self, name: &str, price: u64) -> Result<(), Box<dyn Error>> {
        match self.find_product_id(name) {
            Some(id) => self.change_price(id, price),
            None => Err(StorageError::list(ProductNotFound)),
        }
    }

    pub fn remove_stock(&mut self, id: u64, quantity: usize) -> Result<(), Box<dyn Error>> {
        let step = quantity as isize;
        match self.list.product(id) {
            Some(_) => match self.warehouse.remove_stock(id, quantity) {
                Ok(_) => self.list.step_qty(id, -step),
                Err(e) => Err(e),
            },
            None => Err(StorageError::list(ProductNotFound)),
        }
    }

    pub fn remove_stock_by_name(
        &mut self,
        name: &str,
        quantity: usize,
    ) -> Result<(), Box<dyn Error>> {
        match self.find_product_id(name) {
            Some(id) => self.remove_stock(id, quantity),
            None => Err(StorageError::list(ProductNotFound)),
        }
    }

    pub fn empty_stock(&mut self, id: u64) -> Result<(), Box<dyn Error>> {
        match self.list.product(id) {
            Some(_) => match self.warehouse.remove_all_stock(id) {
                Ok(_) => self.list.empty_qty(id),
                Err(e) => Err(e),
            },
            None => Err(StorageError::list(ProductNotFound)),
        }
    }

    pub fn empty_stock_by_name(&mut self, name: &str) -> Result<(), Box<dyn Error>> {
        match self.find_product_id(name) {
            Some(id) => match self.empty_stock(id) {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            },
            None => Err(StorageError::list(ProductNotFound)),
        }
    }
}

impl Default for Storage {
    fn default() -> Self {
        Storage::new("default".to_string(), None)
    }
}
