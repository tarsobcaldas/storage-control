#[cfg(test)]
use crate::{ warehouse::{Warehouse, PlacementStrategy::*}, product::ProductList};

#[test]
fn contiguous_restock() {
    let mut warehouse = Warehouse::default();
    let mut product_list = ProductList::default();
    let product_id = product_list.id_from_name("Apple").unwrap();
    match warehouse.independent_restock(product_id, 300, &mut product_list, None) {
        Ok(_) => warehouse.print_items_and_names(&product_list),
        Err(e) => panic!("{}", e),
    }
    println!("Product list: {:#?}", product_list);
    println!("{:#?}", warehouse);
}

#[test]
fn round_robin_restock() {
    let mut warehouse = Warehouse { strategy: RoundRobin, ..Warehouse::default() };
    let mut product_list = ProductList::default();
    let product_id = product_list.id_from_name("Apple").unwrap();
    match warehouse.independent_restock(product_id, 100, &mut product_list, None) {
        Ok(_) => warehouse.print_items_and_names(&product_list),
        Err(e) => panic!("{}", e),
    }
    println!("Product list: {:#?}", product_list);
    println!("{:#?}", warehouse);
}

#[test]
fn closest_to_start_restock() {
    let mut warehouse = Warehouse { strategy: ClosestToStart, ..Warehouse::default() };
    let mut product_list = ProductList::default();
    let product_id = product_list.id_from_name("Apple").unwrap();
    match warehouse.independent_restock(product_id, 100, &mut product_list, None) {
        Ok(_) => warehouse.print_items_and_names(&product_list),
        Err(e) => {
            warehouse.print_items_and_names(&product_list);
            panic!("{}", e)
        },
    }
    println!("Product list: {:#?}", product_list);
    println!("{:#?}", warehouse);
}


#[test]
fn contiguous_oversized_restock() {
    let mut warehouse = Warehouse::default();
    let mut product_list = ProductList::default();
    let product_id = product_list.id_from_name("Watermelon").unwrap();
    match warehouse.independent_restock(product_id, 100, &mut product_list, None) {
        Ok(_) => warehouse.print_items_and_names(&product_list),
        Err(e) => panic!("{}", e),
    }
    println!("Product list: {:#?}", product_list);
    println!("{:#?}", warehouse);
}

#[test]
fn round_robin_oversized_restock() {
    let mut warehouse = Warehouse { strategy: RoundRobin, ..Warehouse::default() };
    let mut product_list = ProductList::default();
    let product_id = product_list.id_from_name("Watermelon").unwrap();
    match warehouse.independent_restock(product_id, 100, &mut product_list, None) {
        Ok(_) => warehouse.print_items_and_names(&product_list),
        Err(e) => {
            warehouse.print_items_and_names(&product_list);
            panic!("{}", e)
        },
    }
    println!("Product list: {:#?}", product_list);
    println!("{:?}", warehouse);
}

#[test]
fn closest_to_start_oversized_restock() {
    let mut warehouse = Warehouse { strategy: ClosestToStart, ..Warehouse::default() };
    let mut product_list = ProductList::default();
    let product_id = product_list.id_from_name("Watermelon").unwrap();
    match warehouse.independent_restock(product_id, 100, &mut product_list, None) {
        Ok(_) => warehouse.print_items_and_names(&product_list),
        Err(e) => panic!("{}", e),
    }
    println!("Product list: {:#?}", product_list);
    println!("{:#?}", warehouse);
}

#[test]
fn removal() {
    let mut warehouse = Warehouse::default();
    let mut product_list = ProductList::default();
    let product_id = product_list.id_from_name("Watermelon").unwrap();
    let expiry_date = Some("2021-12-31".parse().unwrap());
    match warehouse.independent_restock(product_id, 50, &mut product_list, expiry_date) {
        Ok(_) => {},
        Err(e) => panic!("{}", e),
    }
    let expiry_date = Some("2022-12-31".parse().unwrap());
    match warehouse.independent_restock(product_id, 50, &mut product_list, expiry_date) {
        Ok(_) => {},
        Err(e) => panic!("{}", e),
    }
    match warehouse.remove_stock(product_id, 50) {
        Ok(_) => warehouse.print_items_and_names(&product_list),
        Err(e) => panic!("{}", e),
    }
    // println!("Product list: {:#?}", product_list);
    // println!("{:#?}", warehouse);
}
