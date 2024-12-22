use crate::product::{ProductItem, ProductList, Quality::*};
use chrono::NaiveDate;
use log::{info, Level as LogLevel, LevelFilter, Metadata, Record, SetLoggerError};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Debug, Display, Formatter},
};
use ErrorMessage::*;
use InfoMessage::*;
use ItemPart::*;
use PlacementStrategy::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ItemPart {
    WholeProduct(ProductItem),
    ProductStart(ProductItem, usize),
    ProductPart(usize, usize),
    ProductEnd(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlacementStrategy {
    Contiguous,
    RoundRobin,
    ClosestToStart,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub number: usize,
    pub item: Option<ItemPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Level {
    pub number: usize,
    pub available_space: usize,
    pub zones: Vec<Zone>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shelf {
    pub number: usize,
    pub available_space: usize,
    pub levels: Vec<Level>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    pub number: usize,
    pub available_space: usize,
    pub shelves: Vec<Shelf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warehouse {
    pub available_space: usize,
    pub rows: Vec<Row>,
    pub strategy: PlacementStrategy,
}

struct WarehouseLogger;

impl log::Log for WarehouseLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= LogLevel::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

static LOGGER: WarehouseLogger = WarehouseLogger;

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Info))
}

#[derive(Debug)]
pub enum InfoMessage {
    Added(String),
    Removed(String),
    Restocked(String),
    Taken(String),
    Moved(String),
    Initialized(String),
}

#[derive(Debug)]
pub enum ErrorMessage {
    InsufficientSpace,
    InsufficientStock,
    NoContiguousSpace,
    NoProductFound,
    NotEnoughZones,
    ZoneOccupied,
    ZoneEmpty,
    ZoneNotFound,
    LevelNotFound,
    ShelfNotFound,
    CannotRemovePart,
    RowNotFound,
    NotProductStart,
    ProductNotListed,
    EndOfRows,
    EndOfWarehouse,
}

impl Display for ErrorMessage {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl ErrorMessage {
    pub(crate) fn as_str(&self) -> &'static str {
        match *self {
            InsufficientSpace => "Insufficient space",
            InsufficientStock => "Insufficient stock",
            NoContiguousSpace => "No contiguous space available to add in bulk. Please organize items first, or add them individually.",
            ZoneOccupied => "Zone is already occupied",
            ZoneEmpty => "Zone is empty",
            ZoneNotFound => "Zone not found",
            LevelNotFound => "Level not found",
            ShelfNotFound => "Shelf not found",
            RowNotFound => "Row not found",
            NoProductFound => "No product found",
            CannotRemovePart => "Cannot remove part without removing whole product",
            NotEnoughZones => "Not enough zones available to fit oversized item",
            NotProductStart => "Zone is not the start of a product",
            ProductNotListed => "Product not listed",
            EndOfRows => "End of last row reached",
            EndOfWarehouse => "End of warehouse reached",
        }
    }

    pub(crate) fn at<T: Debug>(&self, place: T) -> String {
        format!("{} at {:?}", self.as_str(), place)
    }

    pub(crate) fn with_id(&self, product_id: u64) -> String {
        format!("{} — ID {}", self.as_str(), product_id)
    }
}

impl InfoMessage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Added(_) => "Added",
            Removed(_) => "Removed",
            Restocked(_) => "Restocked",
            Initialized(_) => "Initialized",
            Taken(_) => "Taken",
            Moved(_) => "Moved",
        }
    }

    pub fn full_message(&self) -> String {
        match self {
            Added(s) => format!("{} {}", self.as_str(), s),
            Removed(s) => format!("{} {}", self.as_str(), s),
            Restocked(s) => format!("{} {}", self.as_str(), s),
            Initialized(s) => format!("{} {}", self.as_str(), s),
            Taken(s) => format!("{} {}", self.as_str(), s),
            Moved(s) => format!("{} {}", self.as_str(), s),
        }
    }
}

impl Display for InfoMessage {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.full_message())
    }
}

#[derive(Debug)]
struct WarehouseError {
    level: String,
    message: String,
}

impl Display for WarehouseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} Error: {:?}", self.level, self.message)
    }
}

impl Error for WarehouseError {}

impl WarehouseError {
    pub fn boxed(level: &str, message: String) -> Box<dyn Error> {
        Box::new(WarehouseError {
            level: level.to_string(),
            message,
        })
    }

    pub fn addition(message: String) -> Box<dyn Error> {
        WarehouseError::boxed("Adding", message)
    }

    pub fn remotion(message: String) -> Box<dyn Error> {
        WarehouseError::boxed("Remotion", message)
    }

    pub fn placement(message: String) -> Box<dyn Error> {
        WarehouseError::boxed("Placement", message)
    }

    pub fn message(error: ErrorMessage, details: Option<String>) -> String {
        if let Some(details) = details {
            format!("{}: {}", error, details)
        } else {
            format!("{}", error)
        }
    }
}

impl Zone {
    pub fn new(number: usize, item: Option<ItemPart>) -> Self {
        Zone { number, item }
    }

    pub fn add(
        &mut self,
        row_number: usize,
        shelf_number: usize,
        level_number: usize,
        item: ProductItem,
    ) -> Result<(), Box<dyn Error>> {
        if self.item.is_some() {
            let message = ZoneOccupied.at((row_number, shelf_number, level_number, self.number));
            return Err(WarehouseError::addition(message));
        }
        self.item = Some(WholeProduct(item.clone()));
        Ok(())
    }

    pub fn add_part(
        &mut self,
        row_number: usize,
        shelf_number: usize,
        level_number: usize,
        part: ItemPart,
    ) -> Result<(), Box<dyn Error>> {
        if self.item.is_some() {
            let message = ZoneOccupied.at((row_number, shelf_number, level_number, self.number));
            return Err(WarehouseError::addition(message));
        }
        self.item = Some(part);
        Ok(())
    }

    pub fn remove(
        &mut self,
        row_number: usize,
        shelf_number: usize,
        level_number: usize,
        zone_number: usize,
    ) -> Result<(), Box<dyn Error>> {
        if self.item.is_none() {
            let message = ZoneEmpty.at((row_number, shelf_number, level_number, zone_number));
            return Err(WarehouseError::remotion(message));
        }
        match self.item.as_ref().unwrap() {
            WholeProduct(_) => {
                self.item = None;
                Ok(())
            }
            _ => {
                let message =
                    CannotRemovePart.at((row_number, shelf_number, level_number, self.number));
                Err(WarehouseError::remotion(message))
            }
        }
    }

    pub fn remove_part(
        &mut self,
        row_number: usize,
        shelf_number: usize,
        level_number: usize,
    ) -> Result<(), Box<dyn Error>> {
        if self.item.is_none() {
            let message = ZoneEmpty.at((row_number, shelf_number, level_number, self.number));
            return Err(WarehouseError::remotion(message));
        }
        self.item = None;
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.item.is_none()
    }
}

impl Level {
    pub fn new(number: usize) -> Self {
        Level {
            number,
            zones: Vec::new(),
            available_space: 0,
        }
    }

    pub fn add_zone(&mut self, zone: Zone) {
        self.zones.push(zone);
        self.available_space += 1;
    }

    pub fn remove_zone(
        &mut self,
        row_number: usize,
        shelf_number: usize,
        zone_number: usize,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(zone) = self
            .zones
            .iter()
            .position(|zone| zone.number == zone_number)
        {
            self.zones.remove(zone);
            self.available_space -= 1;
            Ok(())
        } else {
            let message = ZoneNotFound.at((row_number, shelf_number, self.number, zone_number));
            Err(WarehouseError::remotion(message))
        }
    }

    pub fn zone(&self, zone_number: usize) -> Option<&Zone> {
        self.zones.iter().find(|zone| zone.number == zone_number)
    }

    pub fn zone_mut(&mut self, zone_number: usize) -> Option<&mut Zone> {
        self.zones
            .iter_mut()
            .find(|zone| zone.number == zone_number)
    }

    pub fn flat_map(&self) -> String {
        self.zones
            .iter()
            .map(|zone| if zone.is_empty() { "0" } else { "1" })
            .collect()
    }

    pub fn oversized_flat_map(&self, zones_required: usize) -> String {
        let binding = self.flat_map();
        let map = binding.as_str();
        let mut oversized_map = String::new();

        let mut index = 0;
        while index + zones_required < map.len() {
            if map[index..index + zones_required] == "0".repeat(zones_required) {
                oversized_map.push('1');
                oversized_map.push_str("0".repeat(zones_required - 1).as_str());
                index += zones_required;
            } else {
                oversized_map.push(map.chars().nth(index).unwrap());
                index += 1;
            }
        }
        oversized_map
    }

    pub fn flat_map_position_to_zone(&self, position: usize) -> Option<usize> {
        if position < self.zones.len() {
            return Some(position)
        } 
        None
    }

    pub fn oversized_flat_map_position_to_zone(
        &self,
        position: usize,
        zones_required: usize,
    ) -> Option<usize> {
        if position < self.zones.len() - zones_required {
            return Some(position)
        }
        None
    }

    pub fn is_full(&self) -> bool {
        self.available_space == 0
    }

    pub fn is_empty(&self) -> bool {
        self.flat_map().chars().all(|c| c == '0')
    }

    pub fn initialize_zones(&mut self, zone_count: usize) {
        for i in 1..=zone_count {
            let zone = Zone::new(i, None);
            self.add_zone(zone);
        }
    }

    pub fn add_item(
        &mut self,
        row_number: usize,
        shelf_number: usize,
        zone_number: usize,
        item: ProductItem,
    ) -> Result<(), Box<dyn Error>> {
        let level_number = self.number;
        if let Some(zone) = self.zone_mut(zone_number) {
            match zone.add(row_number, shelf_number, level_number, item) {
                Ok(_) => {
                    self.available_space -= 1;
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            let message = ZoneNotFound.at((row_number, shelf_number, self.number, zone_number));
            Err(WarehouseError::addition(message))
        }
    }

    pub fn remove_item(
        &mut self,
        row_number: usize,
        shelf_number: usize,
        level_number: usize,
        zone_number: usize,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(zone) = self.zone_mut(zone_number) {
            match zone.remove(row_number, shelf_number, level_number, zone_number) {
                Ok(_) => {
                    self.available_space += 1;
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            let message = ZoneNotFound.at((row_number, shelf_number, self.number, zone_number));
            Err(WarehouseError::remotion(message))
        }
    }

    fn check_if_fits(
        &self,
        row_number: usize,
        shelf_number: usize,
        zone_number: usize,
        zones_required: usize,
    ) -> Result<(), Box<dyn Error>> {
        let map = self.flat_map();
        let last_zone = zone_number + zones_required - 1;
        if zone_number > map.len() {
            let message = ZoneNotFound.at((row_number, shelf_number, self.number, zone_number));
            return Err(WarehouseError::addition(message));
        } else if last_zone > map.len() {
            let message =
                InsufficientSpace.at((row_number, shelf_number, self.number, zone_number));
            return Err(WarehouseError::addition(message));
        }
        for i in zone_number..=last_zone {
            if map.chars().nth(i).unwrap() == '1' {
                let message = ZoneOccupied.at((row_number, shelf_number, self.number, i));
                return Err(WarehouseError::addition(message));
            }
        }
        Ok(())
    }

    pub fn add_oversized_item(
        &mut self,
        row_number: usize,
        shelf_number: usize,
        zone_number: usize,
        item: ProductItem,
    ) -> Result<(), Box<dyn Error>> {
        let zones_required = item.zones_required;
        self.check_if_fits(row_number, shelf_number, zone_number, zones_required)?;
        if let Some(zone) = self.zone_mut(zone_number) {
            let last_zone = zone_number + item.zones_required - 1;
            zone.item = Some(ProductStart(item, last_zone));
            for i in zone_number + 1..last_zone {
                if let Some(z) = self.zone_mut(i) {
                    z.item = Some(ProductPart(zone_number, last_zone));
                }
            }
            if let Some(z) = self.zone_mut(last_zone) {
                z.item = Some(ProductEnd(zone_number));
            }
            self.available_space -= zones_required;
            Ok(())
        } else {
            let message = ZoneNotFound.at((row_number, shelf_number, self.number, zone_number));
            Err(WarehouseError::addition(message))
        }
    }

    fn get_oversized_range(&self, zone_number: usize) -> Option<(usize, usize)> {
        if let Some(zone) = self.zone(zone_number) {
            match &zone.item {
                Some(ProductStart(_, last_zone)) => Some((zone_number, *last_zone)),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn remove_oversized_item(
        &mut self,
        row_number: usize,
        shelf_number: usize,
        zone_number: usize,
    ) -> Result<(), Box<dyn Error>> {
        let range = self.get_oversized_range(zone_number);
        if let Some((start, end)) = range {
            for i in start..=end {
                if let Some(zone) = self.zone_mut(i) {
                    zone.item = None;
                }
            }
            self.available_space += end - start + 1;
            Ok(())
        } else {
            let message = NoProductFound.at((row_number, shelf_number, self.number, zone_number));
            Err(WarehouseError::remotion(message))
        }
    }

    fn find_vacant_zone(&self) -> Option<usize> {
        self.zones.iter().position(|zone| zone.is_empty())
    }

    fn find_oversized_vacant_zone(&self, zones_required: usize) -> Option<usize> {
        let map = self.flat_map();
        let mut index = 0;
        while index + zones_required <= map.len() {
            if map[index..index + zones_required] == "0".repeat(zones_required) {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    pub fn item(&self, zone_number: usize) -> Option<&ProductItem> {
        if let Some(zone) = self.zone(zone_number) {
            match &zone.item {
                Some(WholeProduct(item)) => return Some(item),
                Some(ProductStart(item, _)) => return Some(item),
                Some(ProductPart(start, _)) | Some(ProductEnd(start)) => {
                    if let Some(start_zone) = self.zone(*start) {
                        if let Some(ProductStart(item, _)) = &start_zone.item {
                            return Some(item);
                        } else {
                            return None;
                        }
                    } else {
                        return None;
                    }
                }
                None => return None,
            }
        }
        None
    }

    pub fn item_mut(&mut self, zone_number: usize) -> Option<&mut ProductItem> {
        let item_zone_number = match &self.zone(zone_number) {
            Some(zone) => match &zone.item {
                Some(WholeProduct(_)) => zone_number,
                Some(ProductStart(_, _)) => zone_number,
                Some(ProductPart(start, _)) | Some(ProductEnd(start)) => *start,
                None => return None,
            },
            None => return None,
        };
        if let Some(zone) = self.zone_mut(item_zone_number) {
            match &mut zone.item {
                Some(WholeProduct(item)) => return Some(item),
                Some(ProductStart(item, _)) => return Some(item),
                _ => return None,
            }
        }
        None
    }

    pub fn contains_product(&self, product_id: u64) -> bool {
        self.zones.iter().any(|zone| {
            if let Some(item) = &zone.item {
                match item {
                    WholeProduct(item) => item.id == product_id,
                    ProductStart(item, _) => item.id == product_id,
                    _ => false,
                }
            } else {
                false
            }
        })
    }

    pub fn check_capacity(&self) -> usize {
        self.zones.len()
    }

    pub fn check_oversized_capacity(&self, zones_required: usize) -> usize {
        self.zones.len() / zones_required
    }

    pub fn find_first_item_occurrence(&self, product_id: u64) -> Option<usize> {
        self.zones.iter().position(|zone| {
            if let Some(item) = &zone.item {
                match item {
                    WholeProduct(item) => item.id == product_id,
                    ProductStart(item, _) => item.id == product_id,
                    _ => false,
                }
            } else {
                false
            }
        })
    }

    pub fn find_last_item_occurrence(&self, product_id: u64) -> Option<usize> {
        self.zones.iter().rposition(|zone| {
            if let Some(item) = &zone.item {
                match item {
                    WholeProduct(item) => item.id == product_id,
                    ProductStart(item, _) => item.id == product_id,
                    _ => false,
                }
            } else {
                false
            }
        })
    }

    pub fn find_all_item_occurences(&self, product_id: u64) -> Vec<usize> {
        self.zones
            .iter()
            .enumerate()
            .filter_map(|(i, zone)| {
                if let Some(item) = &zone.item {
                    match item {
                        WholeProduct(item) => {
                            if item.id == product_id {
                                return Some(i);
                            }
                            None
                        }
                        ProductStart(item, _) => {
                            if item.id == product_id {
                                return Some(i);
                            }
                            None
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn items(&self) -> Vec<ProductItem> {
        self.zones
            .iter()
            .filter_map(|zone| {
                if let Some(item) = &zone.item {
                    match item {
                        WholeProduct(item) => Some(item.clone()),
                        ProductStart(item, _) => Some(item.clone()),
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Shelf {
    pub fn new(number: usize) -> Self {
        Shelf {
            number,
            available_space: 0,
            levels: Vec::new(),
        }
    }

    pub fn add_level(&mut self, level: Level) {
        self.available_space += level.available_space;
        self.levels.push(level);
    }

    pub fn remove_level(
        &mut self,
        row_number: usize,
        level_number: usize,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(column) = self
            .levels
            .iter()
            .position(|lvl| lvl.number == level_number)
        {
            self.levels.remove(column);
            self.available_space -= self.levels[column].available_space;
            Ok(())
        } else {
            let message = LevelNotFound.at((row_number, level_number));
            Err(WarehouseError::remotion(message))
        }
    }

    pub fn zone(&self, level_number: usize, zone_number: usize) -> Option<&Zone> {
        if let Some(level) = self.level(level_number) {
            return level.zone(zone_number);
        }
        None
    }

    pub fn zone_mut(&mut self, level_number: usize, zone_number: usize) -> Option<&mut Zone> {
        if let Some(level) = self.level_mut(level_number) {
            return level.zone_mut(zone_number);
        }
        None
    }

    pub fn level(&self, level_number: usize) -> Option<&Level> {
        self.levels.iter().find(|lvl| lvl.number == level_number)
    }

    pub fn level_mut(&mut self, level_number: usize) -> Option<&mut Level> {
        self.levels
            .iter_mut()
            .find(|lvl| lvl.number == level_number)
    }

    pub fn flat_map(&self) -> String {
        self.levels
            .iter()
            .map(|lvl| lvl.flat_map())
            .collect::<Vec<String>>()
            .join("")
    }

    pub fn oversized_flat_map(&self, zones_required: usize) -> String {
        self.levels
            .iter()
            .map(|lvl| lvl.oversized_flat_map(zones_required))
            .collect::<Vec<String>>()
            .join("")
    }

    pub fn flat_map_position_to_zone(&self, position: usize) -> Option<(usize, usize)> {
        let mut cumulative_capacity = 0;
        for (level_index, level) in self.levels.iter().enumerate() {
            let level_capacity = level.check_capacity();
            if position < cumulative_capacity + level_capacity {
                return level
                    .flat_map_position_to_zone(position - cumulative_capacity)
                    .map(|zone_index| (level_index + 1, zone_index));
            }
            cumulative_capacity += level_capacity;
        }
        None
    }

    pub fn oversized_flat_map_position_to_zone(
        &self,
        position: usize,
        zones_required: usize,
    ) -> Option<(usize, usize)> {
        let mut cumulative_capacity = 0;
        for (level_index, level) in self.levels.iter().enumerate() {
            let level_capacity = level.check_capacity();
            println!("{} {}", position, cumulative_capacity);
            if position < cumulative_capacity + level_capacity - zones_required {
                println!("{} {}", position, cumulative_capacity);
                return level
                    .oversized_flat_map_position_to_zone(
                        position - cumulative_capacity,
                        zones_required,
                    )
                    .map(|zone_index| (level_index + 1, zone_index));
            } else if position < cumulative_capacity + level_capacity {
                return level.oversized_flat_map_position_to_zone(
                    1,
                    zones_required,
                ).map(|zone_index| (level_index + 1, zone_index));
            }
            cumulative_capacity += level_capacity;
        }
        None
    }

    pub fn find_vacant_zone(&self) -> Option<(usize, usize)> {
        for (level_index, level) in self.levels.iter().enumerate() {
            if let Some(zone_index) = level.find_vacant_zone() {
                return Some((level_index + 1, zone_index + 1));
            }
        }
        None
    }

    pub fn find_oversized_vacant_zone(&self, zones_required: usize) -> Option<(usize, usize)> {
        for (level_index, level) in self.levels.iter().enumerate() {
            if let Some(zone_index) = level.find_oversized_vacant_zone(zones_required) {
                return Some((level_index + 1, zone_index + 1));
            }
        }
        None
    }

    pub fn contains_product(&self, product_id: u64) -> bool {
        self.levels
            .iter()
            .any(|lvl| lvl.contains_product(product_id))
    }

    pub fn check_capacity(&self) -> usize {
        self.levels.iter().map(|lvl| lvl.check_capacity()).sum()
    }

    pub fn check_oversized_capacity(&self, zones_required: usize) -> usize {
        self.levels
            .iter()
            .map(|lvl| lvl.check_oversized_capacity(zones_required))
            .sum()
    }

    pub fn is_full(&self) -> bool {
        self.available_space == 0
    }

    pub fn is_empty(&self) -> bool {
        self.levels.iter().all(|lvl| lvl.is_empty())
    }

    pub fn initialize_columns(&mut self, level_count: usize, zone_per_level: usize) {
        for i in 1..=level_count {
            let mut column = Level::new(i);
            column.initialize_zones(zone_per_level);
            self.add_level(column);
        }
    }

    pub fn add_item(
        &mut self,
        row_number: usize,
        level_number: usize,
        zone_number: usize,
        item: ProductItem,
    ) -> Result<(), Box<dyn Error>> {
        let shelf_number = self.number;
        if let Some(level) = self.level_mut(level_number) {
            match level.add_item(row_number, shelf_number, zone_number, item) {
                Ok(_) => {
                    self.available_space -= 1;
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            let message = LevelNotFound.at((row_number, shelf_number, level_number));
            Err(WarehouseError::addition(message))
        }
    }

    pub fn remove_item(
        &mut self,
        row_number: usize,
        shelf_number: usize,
        level_number: usize,
        zone_number: usize,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(level) = self.level_mut(level_number) {
            match level.remove_item(row_number, shelf_number, level_number, zone_number) {
                Ok(_) => {
                    self.available_space += 1;
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            let message = LevelNotFound.at((row_number, shelf_number, level_number));
            Err(WarehouseError::remotion(message))
        }
    }

    pub fn add_oversized_item(
        &mut self,
        row_number: usize,
        level_number: usize,
        zone_number: usize,
        item: ProductItem,
    ) -> Result<(), Box<dyn Error>> {
        let shelf_number = self.number;
        let zones_required = item.zones_required;
        if let Some(level) = self.level_mut(level_number) {
            match level.add_oversized_item(row_number, shelf_number, zone_number, item) {
                Ok(_) => {
                    self.available_space -= zones_required;
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            let message = LevelNotFound.at((row_number, self.number, level_number));
            Err(WarehouseError::addition(message))
        }
    }

    pub fn remove_oversized_item(
        &mut self,
        row_number: usize,
        level_number: usize,
        zone_number: usize,
    ) -> Result<(), Box<dyn Error>> {
        let shelf_number = self.number;
        let zones_required = match self.item(level_number, zone_number) {
            Some(item) => item.zones_required,
            None => {
                let message =
                    NoProductFound.at((row_number, shelf_number, level_number, zone_number));
                return Err(WarehouseError::remotion(message));
            }
        };
        if let Some(level) = self.level_mut(level_number) {
            match level.remove_oversized_item(row_number, shelf_number, zone_number) {
                Ok(_) => {
                    self.available_space += zones_required;
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            let message = LevelNotFound.at((row_number, shelf_number, level_number));
            Err(WarehouseError::remotion(message))
        }
    }

    pub fn item(&self, level_number: usize, zone_number: usize) -> Option<&ProductItem> {
        if let Some(level) = self.level(level_number) {
            return level.item(zone_number);
        }
        None
    }

    pub fn item_mut(
        &mut self,
        level_number: usize,
        zone_number: usize,
    ) -> Option<&mut ProductItem> {
        if let Some(level) = self.level_mut(level_number) {
            return level.item_mut(zone_number);
        }
        None
    }

    pub fn find_first_item_occurrence(&self, product_id: u64) -> Option<(usize, usize)> {
        if let Some(level) = self
            .levels
            .iter()
            .find(|lvl| lvl.contains_product(product_id))
        {
            let lvl_index = level.number;
            if let Some(zone_index) = level.find_first_item_occurrence(product_id) {
                return Some((lvl_index, zone_index));
            }
        }
        None
    }

    pub fn find_last_item_occurrence(&self, product_id: u64) -> Option<(usize, usize)> {
        if let Some(level) = self
            .levels
            .iter()
            .rfind(|lvl| lvl.contains_product(product_id))
        {
            let lvl_index = level.number;
            if let Some(zone_index) = level.find_last_item_occurrence(product_id) {
                return Some((lvl_index, zone_index));
            }
        }
        None
    }

    pub fn find_all_item_occurences(&self, product_id: u64) -> Vec<(usize, usize)> {
        let mut items = Vec::new();
        for (lvl_index, level) in self.levels.iter().enumerate() {
            items.extend(
                level
                    .find_all_item_occurences(product_id)
                    .iter()
                    .map(|zone_index| (lvl_index, *zone_index)),
            );
        }
        items
    }

    pub fn items(&self) -> Vec<ProductItem> {
        self.levels.iter().flat_map(|lvl| lvl.items()).collect()
    }
}

impl Row {
    pub fn new(number: usize) -> Self {
        Row {
            number,
            available_space: 0,
            shelves: Vec::new(),
        }
    }

    pub fn add_shelf(&mut self, shelf: Shelf) {
        self.available_space += shelf.available_space;
        self.shelves.push(shelf);
    }

    pub fn remove_shelf(&mut self, shelf_number: usize) -> Result<(), Box<dyn Error>> {
        if let Some(shelf) = self.shelves.iter().position(|sh| sh.number == shelf_number) {
            self.shelves.remove(shelf);
            self.available_space -= self.shelves[shelf].available_space;
            Ok(())
        } else {
            let message = ShelfNotFound.at((self.number, shelf_number));
            Err(WarehouseError::remotion(message))
        }
    }

    pub fn zone(
        &self,
        shelf_number: usize,
        level_number: usize,
        zone_number: usize,
    ) -> Option<&Zone> {
        if let Some(shelf) = self.shelf(shelf_number) {
            if let Some(level) = shelf.level(level_number) {
                return level.zone(zone_number);
            }
        }
        None
    }

    pub fn zone_mut(
        &mut self,
        shelf_number: usize,
        level_number: usize,
        zone_number: usize,
    ) -> Option<&mut Zone> {
        if let Some(shelf) = self.shelf_mut(shelf_number) {
            return shelf.zone_mut(level_number, zone_number);
        }
        None
    }

    pub fn shelf(&self, shelf_number: usize) -> Option<&Shelf> {
        self.shelves.iter().find(|sh| sh.number == shelf_number)
    }

    pub fn shelf_mut(&mut self, shelf_number: usize) -> Option<&mut Shelf> {
        self.shelves.iter_mut().find(|sh| sh.number == shelf_number)
    }

    pub fn flat_map(&self) -> String {
        self.shelves
            .iter()
            .map(|sh| sh.flat_map())
            .collect::<Vec<String>>()
            .join("")
    }

    pub fn oversized_flat_map(&self, zones_required: usize) -> String {
        self.shelves
            .iter()
            .map(|sh| sh.oversized_flat_map(zones_required))
            .collect::<Vec<String>>()
            .join("")
    }

    pub fn flat_map_position_to_zone(&self, position: usize) -> Option<(usize, usize, usize)> {
        let mut cumulative_capacity = 0;
        for (shelf_index, shelf) in self.shelves.iter().enumerate() {
            let shelf_capacity = shelf.check_capacity();
            if position < cumulative_capacity + shelf_capacity {
                return shelf
                    .flat_map_position_to_zone(position - cumulative_capacity)
                    .map(|(level_index, zone_index)| (shelf_index + 1, level_index, zone_index));
            }
            cumulative_capacity += shelf_capacity;
        }
        None
    }

    pub fn oversized_flat_map_position_to_zone(
        &self,
        position: usize,
        zones_required: usize,
    ) -> Option<(usize, usize, usize)> {
        let mut cumulative_capacity = 0;
        for (shelf_index, shelf) in self.shelves.iter().enumerate() {
            let shelf_capacity = shelf.check_capacity();
            if position < cumulative_capacity + shelf_capacity - zones_required {
                println!("{} {}", position, cumulative_capacity);
                return shelf
                    .oversized_flat_map_position_to_zone(
                        position - cumulative_capacity,
                        zones_required,
                    )
                    .map(|(level_index, zone_index)| (shelf_index + 1, level_index, zone_index));
            } else if position < cumulative_capacity + shelf_capacity {
                return shelf.oversized_flat_map_position_to_zone(
                    1,
                    zones_required,
                ).map(|(level_index, zone_index)| (shelf_index + 1, level_index, zone_index));
            }
            cumulative_capacity += shelf_capacity;
        }
        None
    }

    pub fn contains_product(&self, product_id: u64) -> bool {
        self.shelves
            .iter()
            .any(|sh| sh.contains_product(product_id))
    }

    pub fn check_capacity(&self) -> usize {
        self.shelves.iter().map(|sh| sh.check_capacity()).sum()
    }

    pub fn check_oversized_capacity(&self, zones_required: usize) -> usize {
        self.shelves
            .iter()
            .map(|sh| sh.check_oversized_capacity(zones_required))
            .sum()
    }

    pub fn is_full(&self) -> bool {
        self.available_space == 0
    }

    pub fn is_empty(&self) -> bool {
        self.shelves.iter().all(|sh| sh.is_empty())
    }

    pub fn initialize_shelves(
        &mut self,
        shelf_count: usize,
        level_per_shelf: usize,
        zone_per_level: usize,
    ) {
        for i in 1..=shelf_count {
            let mut shelf = Shelf::new(i);
            shelf.initialize_columns(level_per_shelf, zone_per_level);
            self.add_shelf(shelf);
        }
    }

    pub fn add_item(
        &mut self,
        shelf_number: usize,
        level_number: usize,
        zone_number: usize,
        item: ProductItem,
    ) -> Result<(), Box<dyn Error>> {
        let row_number = self.number;
        if let Some(shelf) = self.shelf_mut(shelf_number) {
            match shelf.add_item(row_number, level_number, zone_number, item) {
                Ok(_) => {
                    self.available_space -= 1;
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            let message = ShelfNotFound.at((row_number, shelf_number));
            Err(WarehouseError::addition(message))
        }
    }

    pub fn add_oversized_item(
        &mut self,
        shelf_number: usize,
        level_number: usize,
        zone_number: usize,
        item: ProductItem,
    ) -> Result<(), Box<dyn Error>> {
        let row_number = self.number;
        let zones_required = item.zones_required;
        if let Some(shelf) = self.shelf_mut(shelf_number) {
            match shelf.add_oversized_item(row_number, level_number, zone_number, item) {
                Ok(_) => {
                    self.available_space -= zones_required;
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            let message = ShelfNotFound.at((row_number, shelf_number));
            Err(WarehouseError::addition(message))
        }
    }

    pub fn remove_item(
        &mut self,
        row_number: usize,
        shelf_number: usize,
        level_number: usize,
        zone_number: usize,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(shelf) = self.shelf_mut(shelf_number) {
            match shelf.remove_item(row_number, shelf_number, level_number, zone_number) {
                Ok(_) => {
                    self.available_space += 1;
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            let message = ShelfNotFound.at((row_number, shelf_number));
            Err(WarehouseError::remotion(message))
        }
    }

    pub fn remove_oversized_item(
        &mut self,
        shelf_number: usize,
        level_number: usize,
        zone_number: usize,
    ) -> Result<(), Box<dyn Error>> {
        let row_number = self.number;
        let zones_required = match self.item(shelf_number, level_number, zone_number) {
            Some(item) => item.zones_required,
            None => {
                let message =
                    NoProductFound.at((row_number, shelf_number, level_number, zone_number));
                return Err(WarehouseError::remotion(message));
            }
        };
        if let Some(shelf) = self.shelf_mut(shelf_number) {
            match shelf.remove_oversized_item(row_number, level_number, zone_number) {
                Ok(_) => {
                    self.available_space += zones_required;
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            let message = ShelfNotFound.at((row_number, shelf_number));
            Err(WarehouseError::remotion(message))
        }
    }

    pub fn add_qty(
        &mut self,
        id: u64,
        list: &mut ProductList,
        qty: &mut usize,
        expiry_date: Option<NaiveDate>,
        start: (usize, usize, usize),
    ) -> Result<(), Box<dyn Error>> {
        let product = match list.product(id) {
            Some(product) => product,
            None => {
                let message = WarehouseError::message(ProductNotListed, None);
                return Err(WarehouseError::addition(message));
            }
        };
        let max_level = product.max_level();
        let row = self.number;
        let (mut shelf, mut level, mut zone) = start;
        if let Some(max_level) = max_level {
            if level > max_level {
                shelf += 1;
                level = 1;
                zone = 1;
            }
            if shelf > self.shelves.len() {
                return Ok(());
            }
        }
        while *qty > 0 {
            let placement = (row, shelf, level, zone);
            let item = ProductItem::new(id, list, placement, expiry_date)?;
            match self.add_item(shelf, level, zone, item) {
                Ok(_) => {
                    info!(
                        "{}",
                        Added(format!("{} at {:?}", id, (row, shelf, level, zone)))
                    );
                    *qty -= 1;
                    zone += 1;
                    if zone > self.shelves[shelf - 1].levels[level - 1].zones.len() {
                        zone = 1;
                        level += 1;
                        if level > max_level.unwrap_or(self.shelves[shelf - 1].levels.len()) {
                            level = 1;
                            shelf += 1;
                            if shelf > self.shelves.len() {
                                return Ok(());
                            }
                        }
                    }
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    pub fn add_oversized_qty(
        &mut self,
        id: u64,
        list: &mut ProductList,
        qty: &mut usize,
        expiry_date: Option<NaiveDate>,
        zones_required: usize,
        start: (usize, usize, usize),
    ) -> Result<(), Box<dyn Error>> {
        let product = match list.product(id) {
            Some(product) => product,
            None => {
                let message = WarehouseError::message(ProductNotListed, None);
                return Err(WarehouseError::addition(message));
            }
        };
        let max_level = product.max_level();
        let row = self.number;
        let (mut shelf, mut level, mut zone) = start;
        if let Some(max_level) = max_level {
            if level > max_level {
                shelf += 1;
                level = 1;
                zone = 1;
            }
            if shelf > self.shelves.len() {
                return Ok(());
            }
        }
        while *qty > 0 {
            let placement = (row, shelf, level, zone);
            let item = ProductItem::new(id, list, placement, expiry_date)?;
            match self.add_oversized_item(shelf, level, zone, item) {
                Ok(_) => {
                    info!(
                        "{}",
                        Added(format!("{} at {:?}", id, (row, shelf, level, zone)))
                    );
                    *qty -= 1;
                    zone += zones_required;
                    if zone > self.shelves[shelf - 1].levels[level - 1].zones.len() - zones_required
                    {
                        zone = 1;
                        level += 1;
                        if level > max_level.unwrap_or(self.shelves[shelf - 1].levels.len()) {
                            level = 1;
                            shelf += 1;
                            if shelf > self.shelves.len() {
                                return Ok(());
                            }
                        }
                    }
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    pub fn item(
        &self,
        shelf_number: usize,
        level_number: usize,
        zone_number: usize,
    ) -> Option<&ProductItem> {
        if let Some(shelf) = self.shelf(shelf_number) {
            return shelf.item(level_number, zone_number);
        }
        None
    }

    pub fn item_mut(
        &mut self,
        shelf_number: usize,
        level_number: usize,
        zone_number: usize,
    ) -> Option<&mut ProductItem> {
        if let Some(shelf) = self.shelf_mut(shelf_number) {
            return shelf.item_mut(level_number, zone_number);
        }
        None
    }

    pub fn find_first_item_occurrence(&self, product_id: u64) -> Option<(usize, usize, usize)> {
        for (shelf_index, shelf) in self.shelves.iter().enumerate() {
            if let Some((level_index, zone_index)) = shelf.find_first_item_occurrence(product_id) {
                return Some((shelf_index, level_index, zone_index));
            }
        }
        None
    }

    pub fn find_last_item_occurrence(&self, product_id: u64) -> Option<(usize, usize, usize)> {
        for (shelf_index, shelf) in self.shelves.iter().enumerate().rev() {
            if let Some((level_index, zone_index)) = shelf.find_last_item_occurrence(product_id) {
                return Some((shelf_index, level_index, zone_index));
            }
        }
        None
    }

    pub fn find_all_item_occurences(&self, product_id: u64) -> Vec<(usize, usize, usize)> {
        let mut items = Vec::new();
        for (shelf_index, shelf) in self.shelves.iter().enumerate() {
            items.extend(
                shelf
                    .find_all_item_occurences(product_id)
                    .iter()
                    .map(|(level_index, zone_index)| (shelf_index, *level_index, *zone_index)),
            );
        }
        items
    }

    pub fn items(&self) -> Vec<ProductItem> {
        self.shelves.iter().flat_map(|sh| sh.items()).collect()
    }
}

impl Warehouse {
    pub fn new() -> Self {
        Warehouse {
            available_space: 0,
            rows: Vec::new(),
            strategy: Contiguous,
        }
    }

    pub fn add_row(&mut self, row: Row) {
        self.available_space += row.available_space;
        self.rows.push(row);
    }

    pub fn remove_row(&mut self, row_number: usize) -> Result<(), Box<dyn Error>> {
        if let Some(row_index) = self.rows.iter().position(|r| r.number == row_number) {
            let row = &self.rows[row_index];
            self.available_space -= row.available_space;
            self.rows.remove(row_index);
            Ok(())
        } else {
            let message = WarehouseError::message(RowNotFound, None);
            Err(WarehouseError::remotion(message))
        }
    }

    pub fn zone(
        &self,
        row_number: usize,
        shelf_number: usize,
        level_number: usize,
        zone_number: usize,
    ) -> Option<&Zone> {
        if let Some(row) = self.row(row_number) {
            return row.zone(shelf_number, level_number, zone_number);
        }
        None
    }

    pub fn zone_mut(
        &mut self,
        row_number: usize,
        shelf_number: usize,
        level_number: usize,
        zone_number: usize,
    ) -> Option<&mut Zone> {
        if let Some(row) = self.row_mut(row_number) {
            return row.zone_mut(shelf_number, level_number, zone_number);
        }
        None
    }

    pub fn row(&self, row_number: usize) -> Option<&Row> {
        self.rows.iter().find(|r| r.number == row_number)
    }

    pub fn row_mut(&mut self, row_number: usize) -> Option<&mut Row> {
        self.rows.iter_mut().find(|r| r.number == row_number)
    }

    pub fn check_capacity(&self) -> usize {
        self.rows.iter().map(|r| r.check_capacity()).sum()
    }

    pub fn check_oversized_capacity(&self, zones_required: usize) -> usize {
        self.rows.iter().map(|r| r.check_oversized_capacity(zones_required)).sum()
    }

    pub fn is_full(&self) -> bool {
        self.available_space == 0
    }

    pub fn is_empty(&self) -> bool {
        self.rows.iter().all(|r| r.is_empty())
    }

    pub fn flat_map(&self) -> String {
        self.rows
            .iter()
            .map(|row| row.flat_map())
            .collect::<Vec<String>>()
            .join("")
    }

    pub fn oversized_flat_map(&self, zones_required: usize) -> String {
        self.rows
            .iter()
            .map(|row| row.oversized_flat_map(zones_required))
            .collect::<Vec<String>>()
            .join("")
    }

    pub fn flat_map_position_to_zone(
        &self,
        position: usize,
    ) -> Option<(usize, usize, usize, usize)> {
        let mut cumulative_capacity = 0;
        for (row_index, row) in self.rows.iter().enumerate() {
            let row_capacity = row.check_capacity();
            if position < cumulative_capacity + row_capacity {
                return row
                    .flat_map_position_to_zone(position - cumulative_capacity)
                    .map(|(shelf_index, level_index, zone_index)| {
                        (row_index + 1, shelf_index, level_index, zone_index)
                    });
            }
            cumulative_capacity += row_capacity;
        }
        None
    }

    pub fn oversized_flat_map_position_to_zone(
        &self,
        position: usize,
        zones_required: usize,
    ) -> Option<(usize, usize, usize, usize)> {
        let mut cumulative_capacity = 0;
        for (row_index, row) in self.rows.iter().enumerate() {
            let row_capacity = row.check_capacity();
            if position < cumulative_capacity + row_capacity - zones_required {
                println!("{} {}", position, cumulative_capacity);
                return row
                    .oversized_flat_map_position_to_zone(
                        position - cumulative_capacity,
                        zones_required,
                    )
                    .map(|(shelf_index, level_index, zone_index)| {
                        (row_index + 1, shelf_index, level_index, zone_index)
                    });
            } else if position < cumulative_capacity + row_capacity {
                return row.oversized_flat_map_position_to_zone(
                    1,
                    zones_required,
                ).map(|(shelf_index, level_index, zone_index)| {
                    (row_index + 1, shelf_index, level_index, zone_index)
                });
            }
            cumulative_capacity += row_capacity;
        }
        None
    }

    pub fn initialize_rows(
        &mut self,
        row_count: usize,
        shelves_per_row: usize,
        levels_per_shelf: usize,
        zone_per_level: usize,
    ) {
        for i in 1..=row_count {
            let mut row = Row::new(i);
            row.initialize_shelves(shelves_per_row, levels_per_shelf, zone_per_level);
            self.add_row(row);
        }
    }

    pub fn contains_product(&self, product_id: u64) -> bool {
        self.rows.iter().any(|row| row.contains_product(product_id))
    }

    pub fn add_item(
        &mut self,
        row_number: usize,
        shelf_number: usize,
        level_number: usize,
        zone_number: usize,
        item: ProductItem,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(row) = self.row_mut(row_number) {
            match row.add_item(shelf_number, level_number, zone_number, item) {
                Ok(_) => {
                    self.available_space -= 1;
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            let details = Some(format!("— {}", row_number));
            let message = WarehouseError::message(RowNotFound, details);
            Err(WarehouseError::addition(message))
        }
    }

    pub fn add_oversized_item(
        &mut self,
        row_number: usize,
        shelf_number: usize,
        level_number: usize,
        zone_number: usize,
        item: ProductItem,
    ) -> Result<(), Box<dyn Error>> {
        let zones_required = item.zones_required;
        if let Some(row) = self.row_mut(row_number) {
            match row.add_oversized_item(shelf_number, level_number, zone_number, item) {
                Ok(_) => {
                    self.available_space -= zones_required;
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            let details = Some(format!("- {}", row_number));
            let message = WarehouseError::message(RowNotFound, details);
            Err(WarehouseError::addition(message))
        }
    }

    pub fn remove_item(
        &mut self,
        row_number: usize,
        shelf_number: usize,
        level_number: usize,
        zone_number: usize,
    ) -> Result<(), Box<dyn Error>> {
        let zones_required = match self.item(row_number, shelf_number, level_number, zone_number) {
            Some(item) => item.zones_required,
            None => {
                let message =
                    NoProductFound.at((row_number, shelf_number, level_number, zone_number));
                return Err(WarehouseError::remotion(message));
            }
        };

        if let Some(row) = self.row_mut(row_number) {
            if zones_required > 1 {
                match row.remove_oversized_item(shelf_number, level_number, zone_number) {
                    Ok(_) => {
                        self.available_space += zones_required;
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            } else {
                match row.remove_item(row_number, shelf_number, level_number, zone_number) {
                    Ok(_) => {
                        self.available_space += 1;
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
        } else {
            let details = Some(format!("- {}", row_number));
            let message = WarehouseError::message(RowNotFound, details);
            Err(WarehouseError::remotion(message))
        }
    }

    pub fn item(
        &self,
        row_number: usize,
        shelf_number: usize,
        level_number: usize,
        zone_number: usize,
    ) -> Option<&ProductItem> {
        if let Some(row) = self.row(row_number) {
            return row.item(shelf_number, level_number, zone_number);
        }
        None
    }

    pub fn item_mut(
        &mut self,
        row_number: usize,
        shelf_number: usize,
        level_number: usize,
        zone_number: usize,
    ) -> Option<&mut ProductItem> {
        if let Some(row) = self.row_mut(row_number) {
            return row.item_mut(shelf_number, level_number, zone_number);
        }
        None
    }

    pub fn find_first_item_occurrence(
        &self,
        product_id: u64,
    ) -> Option<(usize, usize, usize, usize)> {
        for row in &self.rows {
            let row_number = row.number;
            if let Some((shelf_number, level_number, zone_number)) =
                row.find_first_item_occurrence(product_id)
            {
                return Some((row_number, shelf_number, level_number, zone_number));
            }
        }
        None
    }

    pub fn find_last_item_occurrence(
        &self,
        product_id: u64,
    ) -> Option<(usize, usize, usize, usize)> {
        for row in &self.rows {
            let row_number = row.number;
            if let Some((shelf_number, level_number, zone_number)) =
                row.find_last_item_occurrence(product_id)
            {
                return Some((row_number, shelf_number, level_number, zone_number));
            }
        }
        None
    }

    pub fn find_all_item_occurences(&self, product_id: u64) -> Vec<(usize, usize, usize, usize)> {
        let mut items = Vec::new();
        for row in &self.rows {
            let row_number = row.number;
            items.extend(row.find_all_item_occurences(product_id).iter().map(
                |(shelf_number, level_number, zone_number)| {
                    (row_number, *shelf_number, *level_number, *zone_number)
                },
            ));
        }
        items
    }

    pub fn items(&self) -> Vec<ProductItem> {
        self.rows.iter().flat_map(|row| row.items()).collect()
    }

    pub fn items_with_names<'a>(
        &self,
        product_list: &'a ProductList,
        item_list: &'a [ProductItem],
    ) -> Vec<(&'a str, &'a ProductItem)> {
        item_list
            .iter()
            .map(|item| {
                let product = product_list.product(item.id).unwrap();
                (product.name.as_str(), item)
            })
            .collect()
    }

    pub fn items_with_id(&self, product_id: u64) -> Vec<ProductItem> {
        self.rows
            .iter()
            .flat_map(|row| row.items())
            .filter(|item| item.id == product_id)
            .collect()
    }

    pub fn items_with_name(
        &self,
        product_name: &str,
        product_list: &ProductList,
    ) -> Vec<ProductItem> {
        self.rows
            .iter()
            .flat_map(|row| row.items())
            .filter(|item| {
                let product = product_list.product(item.id).unwrap();
                product.name == product_name
            })
            .collect()
    }

    pub fn print_item_list(item_list: &[ProductItem]) {
        item_list.iter().for_each(|item| {
            if let Some(expiry_date) = item.expiry_date {
                println!(
                    "ID: {}, Placement: {:?}, Expiry Date: {}",
                    item.id, item.placement, expiry_date
                );
            } else {
                println!("ID: {}, Placement: {:?}", item.id, item.placement);
            }
        });
        println!();
    }

    pub fn print_items(&self) {
        println!("Listing items on warehouse");
        let items = self.items();
        Warehouse::print_item_list(&items);
        println!();
    }

    pub fn print_items_with_id(&self, product_id: u64) {
        println!("Listing items on warehouse with id {}", product_id);
        let items = self.items_with_id(product_id);
        Warehouse::print_item_list(&items);
        println!();
    }

    pub fn print_items_with_name(&self, product_name: &str, product_list: &ProductList) {
        println!("Listing items on warehouse with name {}", product_name);
        let items = self.items_with_name(product_name, product_list);
        Warehouse::print_item_list(&items);
        println!();
    }

    pub fn print_items_and_names(&self, product_list: &ProductList) {
        println!("Listing items on warehouse");
        let items = self.items();
        let item_list = self.items_with_names(product_list, &items);
        item_list.iter().for_each(|(name, item)| {
            if let Some(expiry_date) = item.expiry_date {
                println!(
                    "Product: {}, ID: {}, Placement: {:?}, Expiry Date: {}",
                    name, item.id, item.placement, expiry_date
                );
            } else {
                println!(
                    "Product: {}, ID: {}, Placement: {:?}",
                    name, item.id, item.placement
                );
            }
        });
        println!();
    }

    pub fn print_expiring_items(&self, product_list: &ProductList, expiry_date: NaiveDate) {
        println!("Listing items on warehouse expiring on {}", expiry_date);
        let items = self.items();
        let expiring_items = Warehouse::filter_by_expiry_date(items, expiry_date);
        let item_list = self.items_with_names(product_list, &expiring_items);
        item_list.iter().for_each(|(name, item)| {
            println!(
                "Product: {}, ID: {}, Placement: {:?}",
                name, item.id, item.placement
            );
        });
        println!();
    }

    pub fn print_expiring_with_id(
        &self,
        product_id: u64,
        product_list: &ProductList,
        expiry_date: NaiveDate,
    ) {
        println!(
            "Listing items on warehouse with id {} expiring on {}",
            product_id, expiry_date
        );
        let items = self.items_with_id(product_id);
        let expiring_items = Warehouse::filter_by_expiry_date(items, expiry_date);
        let item_list = self.items_with_names(product_list, &expiring_items);
        item_list.iter().for_each(|(name, item)| {
            println!(
                "Product: {}, ID: {}, Placement: {:?}",
                name, item.id, item.placement
            );
        });
        println!();
    }

    pub fn print_expiring_with_name(
        &self,
        product_name: &str,
        product_list: &ProductList,
        expiry_date: NaiveDate,
    ) {
        println!(
            "Listing items on warehouse with name {} expiring on {}",
            product_name, expiry_date
        );
        let items = self.items_with_name(product_name, product_list);
        let expiring_items = Warehouse::filter_by_expiry_date(items, expiry_date);
        let item_list = self.items_with_names(product_list, &expiring_items);
        item_list.iter().for_each(|(name, item)| {
            println!(
                "Product: {}, ID: {}, Placement: {:?}",
                name, item.id, item.placement
            );
        });
        println!();
    }

    pub fn print_expired_items(&self, product_list: &ProductList, expiry_date: NaiveDate) {
        println!("Listing items on warehouse expired on {}", expiry_date);
        let items = self.items();
        let expired_items = Warehouse::filter_by_expiry_date(items, expiry_date);
        let item_list = self.items_with_names(product_list, &expired_items);
        item_list.iter().for_each(|(name, item)| {
            println!(
                "Product: {}, ID: {}, Placement: {:?}",
                name, item.id, item.placement
            );
        });
        println!();
    }

    pub fn print_expired_with_id(
        &self,
        product_id: u64,
        product_list: &ProductList,
        expiry_date: NaiveDate,
    ) {
        println!(
            "Listing items on warehouse with id {} expired on {}",
            product_id, expiry_date
        );
        let items = self.items_with_id(product_id);
        let expired_items = Warehouse::filter_by_expiry_date(items, expiry_date);
        let item_list = self.items_with_names(product_list, &expired_items);
        item_list.iter().for_each(|(name, item)| {
            println!(
                "Product: {}, ID: {}, Placement: {:?}",
                name, item.id, item.placement
            );
        });
        println!();
    }

    pub fn print_expired_with_name(
        &self,
        product_name: &str,
        product_list: &ProductList,
        expiry_date: NaiveDate,
    ) {
        println!(
            "Listing items on warehouse with name {} expired on {}",
            product_name, expiry_date
        );
        let items = self.items_with_name(product_name, product_list);
        let expired_items = Warehouse::filter_by_expiry_date(items, expiry_date);
        let item_list = self.items_with_names(product_list, &expired_items);
        item_list.iter().for_each(|(name, item)| {
            println!(
                "Product: {}, ID: {}, Placement: {:?}",
                name, item.id, item.placement
            );
        });
        println!();
    }

    pub fn find_first_contiguous_space(&self, qty: usize) -> Option<(usize, usize, usize, usize)> {
        let flat_map = self.flat_map();
        let mut index = 0;
        while index + qty < flat_map.len() {
            if flat_map[index..index + qty] == "0".repeat(qty) {
                return self.flat_map_position_to_zone(index + 1);
            }
            index += 1;
        }
        None
    }

    pub fn find_first_contiguous_oversized_space(
        &self,
        qty: usize,
        zones_required: usize,
    ) -> Option<(usize, usize, usize, usize)> {
        let flat_map = self.oversized_flat_map(zones_required);
        println!("{}", flat_map);
        let mut index = 0;
        while index + (qty * zones_required) < flat_map.len() {
            if flat_map[index..index + (qty * zones_required)]
                == format!("1{}", "0".repeat(zones_required - 1)).repeat(qty)
            {
                println!("{}", index);
                return self.oversized_flat_map_position_to_zone(index + 1, zones_required);
            }
            index += 1;
        }
        None
    }

    pub fn add_qty(
        &mut self,
        id: u64,
        list: &mut ProductList,
        mut qty: usize,
        expiry_date: Option<NaiveDate>,
        start: (usize, usize, usize, usize),
    ) -> Result<(), Box<dyn Error>> {
        let (mut row, mut shelf, mut level, mut zone) = start;
        while qty > 0 {
            let placement = (shelf, level, zone);
            match self.rows[row - 1].add_qty(id, list, &mut qty, expiry_date, placement) {
                Ok(_) => {
                    if row > self.rows.len() {
                        let message = WarehouseError::message(EndOfRows, None);
                        return Err(WarehouseError::addition(message));
                    }
                    row += 1;
                    shelf = 1;
                    level = 1;
                    zone = 1;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    pub fn add_oversized_qty(
        &mut self,
        id: u64,
        list: &mut ProductList,
        mut qty: usize,
        expiry_date: Option<NaiveDate>,
        zones_required: usize,
        start: (usize, usize, usize, usize),
    ) -> Result<(), Box<dyn Error>> {
        let (mut row, mut shelf, mut level, mut zone) = start;
        println!("{:?}", (row, shelf, level, zone));
        while qty > 0 {
            let placement = (shelf, level, zone);
            match self.rows[row - 1].add_oversized_qty(
                id,
                list,
                &mut qty,
                expiry_date,
                zones_required,
                placement,
            ) {
                Ok(_) => {
                    if row > self.rows.len() {
                        let message = WarehouseError::message(EndOfRows, None);
                        return Err(WarehouseError::addition(message));
                    }
                    row += 1;
                    shelf = 1;
                    level = 1;
                    zone = 1;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    pub fn place_contiguous_stock(
        &mut self,
        id: u64,
        list: &mut ProductList,
        qty: usize,
        expiry_date: Option<NaiveDate>,
    ) -> Result<(), Box<dyn Error>> {
        if let Some((row, shelf, level, zone)) = self.find_first_contiguous_space(qty) {
            self.add_qty(id, list, qty, expiry_date, (row, shelf, level, zone))?
        } else {
            let details = Some("Did not find contiguous space".to_string());
            let message = WarehouseError::message(InsufficientSpace, details);
            return Err(WarehouseError::addition(message));
        }
        Ok(())
    }

    pub fn place_contiguous_oversized_stock(
        &mut self,
        id: u64,
        list: &mut ProductList,
        qty: usize,
        expiry_date: Option<NaiveDate>,
        zones_required: usize,
    ) -> Result<(), Box<dyn Error>> {
        if let Some((row, shelf, level, zone)) =
            self.find_first_contiguous_oversized_space(qty, zones_required)
        {
            println!("{:?}", (row, shelf, level, zone));
            self.add_oversized_qty(
                id,
                list,
                qty,
                expiry_date,
                zones_required,
                (row, shelf, level, zone),
            )?;
            Warehouse::print_item_list(&self.items());
        } else {
            let details = Some("Did not find contiguous space".to_string());
            let message = WarehouseError::message(InsufficientSpace, details);
            return Err(WarehouseError::addition(message));
        }
        Ok(())
    }

    pub fn contiguous_placement(
        &mut self,
        id: u64,
        list: &mut ProductList,
        qty: usize,
        expiry_date: Option<NaiveDate>,
    ) -> Result<(), Box<dyn Error>> {
        let product = match list.product(id) {
            Some(product) => product,
            None => {
                let message = WarehouseError::message(ProductNotListed, None);
                return Err(WarehouseError::addition(message));
            }
        };
        match product.quality {
            Oversized(zones_required) | OversizedAndFragile(zones_required, _) => {
                self.place_contiguous_oversized_stock(id, list, qty, expiry_date, zones_required)
            }
            _ => self.place_contiguous_stock(id, list, qty, expiry_date),
        }
    }

    pub fn shelf_vacancy_map(&self) -> HashMap<(usize, usize), bool> {
        let mut vacancy_map = HashMap::new();
        let _ = &self.rows.iter().for_each(|row| {
            row.shelves.iter().for_each(|shelf| {
                vacancy_map.insert((row.number, shelf.number), !shelf.is_full());
            });
        });
        vacancy_map
    }

    pub fn diagonal_search(
        &self,
        vacancy_map: &HashMap<(usize, usize), bool>,
    ) -> Option<(usize, usize)> {
        let mut diagonal = 1;
        let rows = self.rows.len();
        let shelves = self.rows[0].shelves.len();

        while diagonal < rows + shelves {
            for i in 0..diagonal {
                let row = i;
                let shelf = diagonal - i;

                if row <= rows && shelf <= shelves {
                    if let Some(vacant) = vacancy_map.get(&(row, shelf)) {
                        if *vacant {
                            return Some((row, shelf));
                        }
                    }
                }
            }
            diagonal += 1;
        }

        None
    }

    pub fn find_closest_to_start(
        &self,
        vacancy_map: &mut HashMap<(usize, usize), bool>,
        max_level: Option<usize>,
    ) -> Option<(usize, usize, usize, usize)> {
        while let Some((row, shelf)) = self.diagonal_search(vacancy_map) {
            if let Some((level, zone)) = self.rows[row - 1].shelves[shelf - 1].find_vacant_zone() {
                let levels = self.rows[row - 1].shelves[shelf - 1].levels.len();
                let zones = self.rows[row - 1].shelves[shelf - 1].levels[level - 1]
                    .zones
                    .len();
                if zone >= zones && level == max_level.unwrap_or(levels) {
                    vacancy_map.insert((row, shelf), false);
                } else if level > max_level.unwrap_or(levels) {
                    vacancy_map.insert((row, shelf), false);
                    continue;
                }
                return Some((row, shelf, level, zone));
            }
        }
        None
    }

    pub fn find_oversized_closest_to_start(
        &self,
        vacancy_map: &mut HashMap<(usize, usize), bool>,
        max_level: Option<usize>,
        zones_required: usize,
    ) -> Option<(usize, usize, usize, usize)> {
        while let Some((row, shelf)) = self.diagonal_search(vacancy_map) {
            if let Some((level, zone)) =
                self.rows[row - 1].shelves[shelf - 1].find_oversized_vacant_zone(zones_required)
            {
                let levels = self.rows[row - 1].shelves[shelf - 1].levels.len();
                let zones = self.rows[row - 1].shelves[shelf - 1].levels[level - 1]
                    .zones
                    .len();
                if zone >= zones - zones_required && level == levels {
                    vacancy_map.insert((row, shelf), false);
                } else if level > max_level.unwrap_or(levels) {
                    vacancy_map.insert((row, shelf), false);
                    continue;
                }
                return Some((row, shelf, level, zone));
            }
        }
        None
    }

    pub fn place_stock_closest_to_start(
        &mut self,
        id: u64,
        list: &mut ProductList,
        mut qty: usize,
        expiry_date: Option<NaiveDate>,
    ) -> Result<(), Box<dyn Error>> {
        let mut vacancy_map = self.shelf_vacancy_map();
        let max_level = list.product(id).map(|p| p.max_level()).unwrap();
        while qty > 0 {
            let place = self.find_closest_to_start(&mut vacancy_map, max_level);
            if let Some((row, shelf, level, zone)) = place {
                let placement = (row, shelf, level, zone);
                let item = match ProductItem::new(id, list, placement, expiry_date) {
                    Ok(item) => item,
                    Err(e) => return Err(e),
                };
                match self.add_item(row, shelf, level, zone, item) {
                    Ok(_) => {
                        qty -= 1;
                    }
                    Err(e) => return Err(e),
                }
            } else {
                let details = Some("Did not find space close to start".to_string());
                let message = WarehouseError::message(InsufficientSpace, details);
                return Err(WarehouseError::placement(message));
            }
        }
        Ok(())
    }

    pub fn place_oversized_stock_closest_to_start(
        &mut self,
        id: u64,
        list: &mut ProductList,
        mut qty: usize,
        expiry_date: Option<NaiveDate>,
        zones_required: usize,
    ) -> Result<(), Box<dyn Error>> {
        let mut vacancy_map = self.shelf_vacancy_map();
        let max_level = list.product(id).map(|p| p.max_level()).unwrap();
        while qty > 0 {
            let place =
                self.find_oversized_closest_to_start(&mut vacancy_map, max_level, zones_required);
            if let Some((row, shelf, level, zone)) = place {
                let placement = (row, shelf, level, zone);
                let item = match ProductItem::new(id, list, placement, expiry_date) {
                    Ok(item) => item,
                    Err(e) => return Err(e),
                };
                match self.add_oversized_item(row, shelf, level, zone, item) {
                    Ok(_) => {
                        qty -= 1;
                    }
                    Err(e) => return Err(e),
                }
            } else {
                let details = Some("Did not find space close to start".to_string());
                let message = WarehouseError::message(InsufficientSpace, details);
                return Err(WarehouseError::placement(message));
            }
        }
        Ok(())
    }

    pub fn closest_to_start_placement(
        &mut self,
        id: u64,
        list: &mut ProductList,
        qty: usize,
        expiry_date: Option<NaiveDate>,
    ) -> Result<(), Box<dyn Error>> {
        let product = match list.product(id) {
            Some(product) => product,
            None => {
                return Err(WarehouseError::placement(ProductNotListed.with_id(id)));
            }
        };
        match product.quality {
            Oversized(zones_required) | OversizedAndFragile(zones_required, _) => self
                .place_oversized_stock_closest_to_start(id, list, qty, expiry_date, zones_required),
            _ => self.place_stock_closest_to_start(id, list, qty, expiry_date),
        }
    }

    pub fn find_round_robin_continuation(
        &self,
        flat_map: String,
    ) -> Option<(usize, usize, usize, usize)> {
        if let Some(last_index) = flat_map.rfind('1') {
            self.flat_map_position_to_zone(last_index)
        } else {
            Some((1, 1, 1, 1))
        }
    }

    pub fn find_oversized_round_robin_continuation(
        &self,
        flat_map: String,
        zones_required: usize,
    ) -> Option<(usize, usize, usize, usize)> {
        let pattern = "0".repeat(zones_required);
        if let Some(last_index) = flat_map.rfind(pattern.as_str()) {
            self.oversized_flat_map_position_to_zone(last_index, zones_required)
        } else if self.is_empty() {
            Some((1, 1, 1, 1))
        } else {
            None
        }
    }

    pub fn place_stock_in_round_robin(
        &mut self,
        id: u64,
        list: &mut ProductList,
        qty: usize,
        expiry_date: Option<NaiveDate>,
    ) -> Result<(), Box<dyn Error>> {
        let flat_map = self.flat_map();
        if let Some(first_zone) = self.find_round_robin_continuation(flat_map) {
            self.add_qty(id, list, qty, expiry_date, first_zone)?;
            Ok(())
        } else {
            let details = Some("Did not find place to continue round-robin".to_string());
            let message = WarehouseError::message(InsufficientSpace, details);
            Err(WarehouseError::placement(message))
        }
    }

    pub fn place_oversized_stock_in_round_robin(
        &mut self,
        id: u64,
        list: &mut ProductList,
        qty: usize,
        expiry_date: Option<NaiveDate>,
        zones_required: usize,
    ) -> Result<(), Box<dyn Error>> {
        let flat_map = self.oversized_flat_map(zones_required);
        let first_zone = self.find_oversized_round_robin_continuation(flat_map, zones_required);
        if let Some(first_zone) = first_zone {
            self.add_oversized_qty(id, list, qty, expiry_date, zones_required, first_zone)?;
            Ok(())
        } else {
            let details = Some("Did not find place to continue round-robin".to_string());
            let message = WarehouseError::message(InsufficientSpace, details);
            Err(WarehouseError::placement(message))
        }
    }

    pub fn round_robin_placement(
        &mut self,
        id: u64,
        list: &mut ProductList,
        qty: usize,
        expiry_date: Option<NaiveDate>,
    ) -> Result<(), Box<dyn Error>> {
        let product = match list.product(id) {
            Some(product) => product,
            None => {
                return Err(WarehouseError::placement(ProductNotListed.with_id(id)));
            }
        };
        match product.quality {
            Oversized(zones_required) | OversizedAndFragile(zones_required, _) => self
                .place_oversized_stock_in_round_robin(id, list, qty, expiry_date, zones_required),
            _ => self.place_stock_in_round_robin(id, list, qty, expiry_date),
        }
    }

    pub fn independent_restock(
        &mut self,
        id: u64,
        qty: usize,
        list: &mut ProductList,
        expiry_date: Option<NaiveDate>,
    ) -> Result<(), Box<dyn Error>> {
        if list.product(id).is_some() {
            match self.strategy {
                Contiguous => self.contiguous_placement(id, list, qty, expiry_date),
                RoundRobin => self.round_robin_placement(id, list, qty, expiry_date),
                ClosestToStart => self.closest_to_start_placement(id, list, qty, expiry_date),
            }?;
            info!("{}", Restocked(format!("{} units of {}", qty, id)));
            Ok(())
        } else {
            Err(WarehouseError::placement(ProductNotListed.with_id(id)))
        }
    }

    pub fn sort_by_expiry_date(item_list: Vec<ProductItem>) -> Vec<ProductItem> {
        let mut items = item_list.clone();
        items.sort_by(|a, b| {
            a.expiry_date
                .unwrap_or_else(|| NaiveDate::from_ymd_opt(9999, 12, 31).unwrap())
                .cmp(
                    &b.expiry_date
                        .unwrap_or_else(|| NaiveDate::from_ymd_opt(9999, 12, 31).unwrap()),
                )
        });
        items
    }

    pub fn filter_by_expiry_date(
        item_list: Vec<ProductItem>,
        expiry_date: NaiveDate,
    ) -> Vec<ProductItem> {
        item_list
            .into_iter()
            .filter(|item| item.expiry_date > Some(expiry_date))
            .collect()
    }

    pub fn filter_expired_items(&self, expiry_date: NaiveDate) -> Vec<ProductItem> {
        self.items()
            .into_iter()
            .filter(|item| item.expiry_date < Some(expiry_date))
            .collect()
    }

    pub fn take_stock(
        &mut self,
        mut qty: usize,
        mut list: Vec<ProductItem>,
    ) -> Result<Vec<ProductItem>, Box<dyn Error>> {
        let mut taken_items = Vec::new();
        while qty > 0 {
            if let Some(item) = list.pop() {
                let (row, shelf, level, zone) = item.placement;
                match self.remove_item(row, shelf, level, zone) {
                    Ok(_) => {
                        info!("Taken item {}", item);
                        taken_items.push(item);
                        qty -= 1;
                    }
                    Err(e) => return Err(e),
                }
            } else {
                let message = WarehouseError::message(InsufficientStock, None);
                return Err(WarehouseError::remotion(message));
            }
        }
        Ok(taken_items)
    }

    pub fn remove_stock(&mut self, id: u64, qty: usize) -> Result<(), Box<dyn Error>> {
        let mut list = self.items_with_id(id);
        if list[0].expiry_date.is_some() {
            list = Warehouse::sort_by_expiry_date(list);
            list.reverse();
        }
        self.take_stock(qty, list)?;
        info!("{}", Removed(format!("{} units of {}", qty, id)));
        Ok(())
    }

    pub fn remove_all_stock(&mut self, id: u64) -> Result<(), Box<dyn Error>> {
        let list = self.items_with_id(id);
        self.remove_stock(id, list.len())?;
        Ok(())
    }

    pub fn empty_warehouse(&mut self) -> Result<(), Box<dyn Error>> {
        let list = self.items();
        let _ = self.take_stock(list.len(), list)?;
        Ok(())
    }
}

impl Default for Warehouse {
    fn default() -> Self {
        let mut warehouse = Warehouse::new();
        warehouse.initialize_rows(2, 6, 4, 10);
        warehouse
    }
}
