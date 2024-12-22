use {
    crate::{
        inventory::Storage,
        product::Quality,
        warehouse::Warehouse,
    },
    chrono::NaiveDate,
    clap::{crate_name, Args, Parser, Subcommand},
    std::{
        error::Error,
        fmt::{self, Display, Formatter},
        io::{stdin, stdout, Write},
        path::Path,
    },
    ErrorMessage::*,
};

struct Prompt;

struct Parsing;

#[derive(Parser, Debug)]
struct Repl {
    #[clap(subcommand)]
    cmd: Commands,
}

#[derive(Parser, Debug)]
pub struct Cli {
    storage_path: Option<String>,
    #[clap(subcommand)]
    cmd: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(subcommand_required = true)]
    Add {
        name: String,
        price: u64,
        #[clap(subcommand)]
        quality: QualityOptions,
    },
    Delete {
        #[arg(required_unless_present = "name")]
        id: Option<u64>,
        #[arg(long, short)]
        name: Option<String>,
    },
    #[command(subcommand_required = true)]
    Remove {
        #[arg(required_unless_present = "name")]
        id: Option<u64>,
        #[arg(long, short)]
        name: Option<String>,
        #[arg(required = true)]
        quantity: usize,
    },
    #[command(subcommand_required = true)]
    Change(ChangeCommands),
    Restock {
        #[arg(required_unless_present = "name")]
        id: Option<u64>,
        #[arg(long, short)]
        name: Option<String>,
        #[arg(required = true)]
        quantity: usize,
        #[clap(short, long)]
        expiration_date: Option<NaiveDate>,
    },
    List(ListCommands),
    CreateStorage,
    Load {
        file_path: String,
    },
    Save {
        file_path: Option<String>,
    },
    Exit,
    ForceExit,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ErrorMessage {
    InvalidCommand,
    InvalidId,
    InvalidIdOrName,
    InvalidQuantity,
    InvalidPrice,
    InvalidDate,
    InvalidNumber,
    InvalidFile,
    CouldNotSaveStorage,
    CouldNotCreateStorage,
    CouldNotLoadStorage,
    ExpiredAndExpiring,
    InteractiveModeOnly,
}

impl ErrorMessage {
    pub(crate) fn as_str(&self) -> &'static str {
        match *self {
            InvalidCommand => "Invalid command",
            InvalidId => "Invalid ID",
            InvalidIdOrName => "Invalid ID or Name",
            InvalidQuantity => "Invalid quantity",
            InvalidPrice => "Invalid price",
            InvalidDate => "Invalid date",
            InvalidNumber => "Invalid number",
            InvalidFile => "Invalid file",
            CouldNotSaveStorage => "Could not save storage",
            CouldNotCreateStorage => "Could not create storage",
            CouldNotLoadStorage => "Could not load storage",
            ExpiredAndExpiring => "Cannot list expired and expiring items",
            InteractiveModeOnly => "This command can only be used on interactve mode",
        }
    }
}

impl Display for ErrorMessage {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug)]
struct ReplError {
    message: String,
}

impl Display for ReplError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "REPL Error: {}", self.message)
    }
}

impl Error for ReplError {}

impl ReplError {
    pub fn boxed(message: String) -> Box<dyn Error> {
        Box::new(ReplError {
            message: message.to_string(),
        })
    }

    pub fn base(message: ErrorMessage) -> Box<dyn Error> {
        ReplError::boxed(format!("{}", message))
    }
}

fn read_number() -> Result<u64, Box<dyn Error>> {
    let mut input = String::new();
    match stdin().read_line(&mut input) {
        Ok(_) => match input.trim().parse::<u64>() {
            Ok(number) => Ok(number),
            Err(_) => Err(ReplError::base(InvalidNumber)),
        },
        Err(_) => Err(ReplError::base(InvalidNumber)),
    }
}

#[derive(Debug, Args)]
pub struct ChangeCommands {
    #[clap(subcommand)]
    cmd: ChangeSubcommands,
}

#[derive(Debug, Args)]
pub struct ListCommands {
    #[clap(subcommand)]
    cmd: ListSubcommands,
}

#[derive(Debug, Args)]
struct ProductArgs {}

#[derive(Debug, Args)]
struct RowArgs {
    #[clap(short, long)]
    rows: usize,
}

#[derive(Debug, Args)]
struct ShelfArgs {
    #[clap(short, long)]
    shelves: usize,
    row: usize,
}

#[derive(Debug, Subcommand)]
enum ChangeSubcommands {
    Name(NameArgs),
    Price(PriceArgs),
    #[clap(subcommand)]
    Quality(QualityOptions),
}

#[derive(Debug, Args)]
struct NameArgs {
    id: u64,
    name: String,
}

#[derive(Debug, Subcommand)]
enum QualityOptions {
    Normal,
    Oversized(OversizedArgs),
    Fragile(FragileArgs),
    OversizedAndFragile(QualityArgs),
}

#[derive(Debug, Args)]
struct OversizedArgs {
    zones: usize,
}

#[derive(Debug, Args)]
struct FragileArgs {
    level: usize,
}

#[derive(Debug, Args)]
struct QualityArgs {
    zones: usize,
    level: usize,
}

#[derive(Debug, Args)]
struct PriceArgs {
    #[arg(required_unless_present = "name")]
    id: Option<u64>,
    #[arg(long, short)]
    name: Option<String>,
    price: u64,
}

#[derive(Debug, Subcommand)]
enum ListSubcommands {
    Products(ListProductsArgs),
    Items(ListItemsArgs),
}

#[derive(Debug, Args)]
struct ListItemsArgs {
    #[clap(short, long)]
    name: Option<String>,
    #[clap(short, long)]
    id: Option<u64>,
    #[clap(short, long)]
    expiring: Option<u64>,
    #[clap(long)]
    expired: Option<bool>,
}

#[derive(Debug, Args)]
struct ListProductsArgs {
    #[clap(short, long)]
    name: Option<String>,
    #[clap(long)]
    max_price: Option<u64>,
    #[clap(long)]
    min_price: Option<u64>,
    #[clap(short, long)]
    quality: Option<String>,
}

#[allow(dead_code)]
impl Parsing {
    fn price(price: &str) -> Result<u64, Box<dyn Error>> {
        let normalized_price = price.replace(",", ".");

        match normalized_price.parse::<f64>() {
            Ok(parsed_price) => {
                let price_in_cents = (parsed_price * 100.0).round() as u64;
                Ok(price_in_cents)
            }
            Err(_) => Err(ReplError::base(InvalidPrice)),
        }
    }

    fn optional_date(date_str: &str) -> Option<NaiveDate> {
        let formats = [
            "%Y-%m-%d", "%Y/%m/%d", "%Y.%m.%d", "%Y %m %d", "%Y%m%d", "%d-%m-%Y", "%d/%m/%Y",
            "%d.%m.%Y", "%d %m %Y", "%d%m%Y",
        ];

        for format in formats.iter() {
            if let Ok(date) = NaiveDate::parse_from_str(date_str, format) {
                return Some(date);
            }
        }

        None
    }

    fn handle_args(args: Vec<String>, expected_args: usize) -> Result<Vec<String>, &'static str> {
        if args.is_empty() {
            return Err("No arguments provided.");
        }

        if expected_args >= 2 {
            match args[0].parse::<u64>() {
                Ok(_) => {
                    if args.len() >= expected_args {
                        return Ok(args);
                    } else {
                        return Err("Not enough arguments provided.");
                    }
                }
                Err(_) => {
                    if args.len() >= expected_args {
                        return Ok(args);
                    } else {
                        return Err("Not enough arguments provided.");
                    }
                }
            }
        }
        Err("Invalid command or arguments.")
    }
}

#[allow(dead_code)]
impl Prompt {
    fn id() -> Result<u64, Box<dyn Error>> {
        println!("Enter the ID of the product:");
        read_number()
    }

    fn name() -> String {
        println!("Enter the name of the product:");
        let mut name = String::new();
        stdin().read_line(&mut name).unwrap();
        name.trim().to_string()
    }

    fn id_or_name() -> Result<String, Box<dyn Error>> {
        println!("Enter the ID or name of the product:");
        let mut id_or_name = String::new();
        match stdin().read_line(&mut id_or_name) {
            Ok(_) => Ok(id_or_name.trim().to_string()),
            Err(_) => Err(ReplError::base(InvalidIdOrName)),
        }
    }

    fn quantity() -> Result<usize, Box<dyn Error>> {
        println!("Enter the quantity of the product:");
        let mut quantity = String::new();
        match stdin().read_line(&mut quantity) {
            Ok(_) => match quantity.trim().parse::<usize>() {
                Ok(quantity) => Ok(quantity),
                Err(_) => Err(ReplError::base(InvalidQuantity)),
            },
            Err(_) => Err(ReplError::base(InvalidQuantity)),
        }
    }

    fn price() -> Result<u64, Box<dyn Error>> {
        println!("Enter the price of the product:");
        let mut price = String::new();
        match stdin().read_line(&mut price) {
            Ok(_) => match Parsing::price(price.trim()) {
                Ok(price) => Ok(price),
                Err(e) => Err(e),
            },
            Err(_) => Err(ReplError::base(InvalidPrice)),
        }
    }

    fn expiration_date() -> Option<NaiveDate> {
        println!("Enter the expiration date of the product (optional):");
        let mut expiration_date = String::new();
        match stdin().read_line(&mut expiration_date) {
            Ok(_) => Parsing::optional_date(expiration_date.trim()),
            Err(_) => None,
        }
    }

    fn quality() -> Result<Quality, Box<dyn Error>> {
        println!("Enter quality (Oversized, Fragile, Oversized and Fragile  or Normal)");
        let mut quality = String::new();
        let mut args = String::new();
        match stdin().read_line(&mut quality) {
            Ok(_) => match quality.trim() {
                "Oversized" => {
                    print!("Enter zones required");
                    stdout().flush().unwrap();
                    match stdin().read_line(&mut args) {
                        Ok(_) => match args.trim().parse::<usize>() {
                            Ok(zones) => Ok(Quality::Oversized(zones)),
                            Err(_) => Err(ReplError::base(InvalidNumber)),
                        },
                        Err(e) => Err(Box::new(e)),
                    }
                }
                "Fragile" => {
                    print!("Enter max level required");
                    stdout().flush().unwrap();
                    match stdin().read_line(&mut args) {
                        Ok(_) => match args.trim().parse::<usize>() {
                            Ok(level) => Ok(Quality::Fragile(level)),
                            Err(_) => Ok(Quality::Normal),
                        },
                        Err(e) => Err(Box::new(e)),
                    }
                }
                "Oversized and Fragile" => {
                    print!("Enter zones required");
                    stdout().flush().unwrap();
                    match stdin().read_line(&mut args) {
                        Ok(_) => match args.trim().parse::<usize>() {
                            Ok(zones) => {
                                print!("Enter max level required");
                                stdout().flush().unwrap();
                                match stdin().read_line(&mut args) {
                                    Ok(_) => match args.trim().parse::<usize>() {
                                        Ok(level) => Ok(Quality::OversizedAndFragile(zones, level)),
                                        Err(_) => Err(ReplError::base(InvalidNumber)),
                                    },
                                    Err(e) => Err(Box::new(e)),
                                }
                            }
                            Err(_) => Err(ReplError::base(InvalidNumber)),
                        },
                        Err(e) => Err(Box::new(e)),
                    }
                }
                _ => Ok(Quality::Normal),
            },
            Err(_) => Ok(Quality::Normal),
        }
    }

    fn file_path() -> Option<String> {
        println!("Enter the file path for the storage (default: ./storage-<name>.json):");
        let mut file_path = String::new();
        if stdin().read_line(&mut file_path).is_ok() {
            if file_path.trim().is_empty() {
                None
            } else {
                Some(file_path.trim().to_string())
            }
        } else {
            None
        }
    }

    fn warehouse_creation(mut warehouse: Warehouse) -> Result<Warehouse, Box<dyn Error>> {
        print!("Enter the number of rows in the warehouse:");
        stdout().flush().unwrap();
        let rows: usize = match read_number() {
            Ok(number) => number as usize,
            Err(_) => return Err(ReplError::base(InvalidNumber)),
        };

        print!("Enter the number of shelves in each row of the warehouse:");
        stdout().flush().unwrap();
        let shelves: usize = match read_number() {
            Ok(number) => number as usize,
            Err(_) => return Err(ReplError::base(InvalidNumber)),
        };

        print!("Enter the number of levels in each shelf of the warehouse:");
        stdout().flush().unwrap();
        let levels: usize = match read_number() {
            Ok(number) => number as usize,
            Err(_) => return Err(ReplError::base(InvalidNumber)),
        };

        print!("Enter the number of zones in each column of the warehouse:");
        stdout().flush().unwrap();
        let zones: usize = match read_number() {
            Ok(number) => number as usize,
            Err(_) => return Err(ReplError::base(InvalidNumber)),
        };

        warehouse.initialize_rows(rows, shelves, levels, zones);
        Ok(warehouse)
    }

    fn storage_load(storage: &mut Storage) -> Result<&mut Storage, Box<dyn Error>> {
        match Prompt::file_path() {
            Some(file_path) => {
                let default_path_name = format!("./storage-{}.json", &file_path);
                let default_path = Path::new(&default_path_name);
                if !default_path.exists() {
                    match Storage::load(&file_path, storage) {
                        Ok(loaded) => Ok(loaded),
                        Err(e) => Err(e),
                    }
                } else {
                    match Storage::load(&default_path_name, storage) {
                        Ok(loaded) => Ok(loaded),
                        Err(e) => Err(e),
                    }
                }
            }
            None => Err(ReplError::base(InvalidFile)),
        }
    }

    fn storage_creation(storage: &mut Storage) -> Result<&mut Storage, Box<dyn Error>> {
        println!("Enter the name of the storage:");
        let mut name = String::new();
        if stdin().read_line(&mut name).is_ok() {
            name = name.trim().to_string();
        }
        let file_path = Prompt::file_path();

        let warehouse = Warehouse::new();
        storage.file_path = file_path.unwrap_or(format!("./storage-{}.json", name));
        storage.name = name;
        match Prompt::warehouse_creation(warehouse) {
            Ok(warehouse) => {
                storage.warehouse = warehouse;
                Ok(storage)
            }
            Err(e) => Err(e),
        }
    }

    fn new_product(storage: &mut Storage) -> Result<(), Box<dyn Error>> {
        let name = Prompt::name();
        match Prompt::price() {
            Ok(price) => match Prompt::quality() {
                Ok(quality) => match storage.new_product(name, price, quality) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e),
                },
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        }
    }

    fn delete_product(storage: &mut Storage) -> Result<(), Box<dyn Error>> {
        match Prompt::id_or_name() {
            Ok(id_or_name) => match id_or_name.parse::<u64>() {
                Ok(id) => match storage.delete_product_by_id(id) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e),
                },
                Err(_) => match storage.delete_product_by_name(&id_or_name) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e),
                },
            },
            Err(e) => Err(e),
        }
    }

    fn price_change(storage: &mut Storage) -> Result<(), Box<dyn Error>> {
        match Prompt::id() {
            Ok(id) => match Prompt::price() {
                Ok(price) => match storage.change_price(id, price) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e),
                },
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        }
    }

    fn restock_product(storage: &mut Storage) -> Result<(), Box<dyn Error>> {
        match Prompt::id_or_name() {
            Ok(id_or_name) => match id_or_name.parse::<u64>() {
                Ok(id) => match Prompt::quantity() {
                    Ok(quantity) => match Prompt::expiration_date() {
                        Some(expiry) => match storage.restock_product(id, quantity, Some(expiry)) {
                            Ok(_) => Ok(()),
                            Err(e) => Err(e),
                        },
                        None => match storage.restock_product(id, quantity, None) {
                            Ok(_) => Ok(()),
                            Err(e) => Err(e),
                        },
                    },
                    Err(e) => Err(e),
                },
                Err(_) => match Prompt::quantity() {
                    Ok(quantity) => match Prompt::expiration_date() {
                        Some(expiry) => {
                            match storage.restock_by_name(&id_or_name, quantity, Some(expiry)) {
                                Ok(_) => Ok(()),
                                Err(e) => Err(e),
                            }
                        }
                        None => match storage.restock_by_name(&id_or_name, quantity, None) {
                            Ok(_) => Ok(()),
                            Err(e) => Err(e),
                        },
                    },
                    Err(e) => Err(e),
                },
            },
            Err(e) => Err(e),
        }
    }

    fn remove_stock(storage: &mut Storage) -> Result<(), Box<dyn Error>> {
        match Prompt::id_or_name() {
            Ok(id_or_name) => match id_or_name.parse::<u64>() {
                Ok(id) => match Prompt::quantity() {
                    Ok(quantity) => match storage.remove_stock(id, quantity) {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    },
                    Err(e) => Err(e),
                },
                Err(_) => match Prompt::quantity() {
                    Ok(quantity) => match storage.remove_stock_by_name(&id_or_name, quantity) {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    },
                    Err(e) => Err(e),
                },
            },
            Err(e) => Err(e),
        }
    }

    fn empty_stock(storage: &mut Storage) -> Result<(), Box<dyn Error>> {
        match Prompt::id_or_name() {
            Ok(id_or_name) => match id_or_name.parse::<u64>() {
                Ok(id) => match storage.empty_stock(id) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e),
                },
                Err(_) => match storage.empty_stock_by_name(&id_or_name) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e),
                },
            },
            Err(e) => Err(e),
        }
    }
}

fn readline() -> Result<String, Box<dyn Error>> {
    print!("> ");
    stdout().flush().unwrap();
    let mut buffer = String::new();
    match stdin().read_line(&mut buffer) {
        Ok(_) => {
            let trimmed_input = buffer.trim().to_string();
            let line = format!("{} {}", crate_name!(), trimmed_input);
            Ok(line)
        }
        Err(e) => Err(Box::new(e)),
    }
}


fn resolve_cmd(cmd: Commands, storage: &mut Storage) -> Result<bool, Box<dyn Error>> {
    use Commands::*;
    match cmd {
        Add {
            name,
            price,
            quality,
        } => {
            use QualityOptions::*;
            let quality = match quality {
                Normal => Quality::Normal,
                Oversized(OversizedArgs { zones }) => Quality::Oversized(zones),
                Fragile(FragileArgs { level }) => Quality::Fragile(level),
                OversizedAndFragile(QualityArgs { zones, level }) => {
                    Quality::OversizedAndFragile(zones, level)
                }
            };
            storage.new_product(name, price, quality)?;
            Ok(true)
        }
        Delete { id, name } => {
            if let Some(name) = name {
                storage.delete_product_by_name(&name)?;
            } else if let Some(id) = id {
                storage.delete_product_by_id(id)?;
            } else {
                Prompt::delete_product(storage)?;
            }
            Ok(true)
        }
        Remove { id, name, quantity } => {
            if let Some(name) = name {
                storage.remove_stock_by_name(&name, quantity)?;
            } else if let Some(id) = id {
                storage.remove_stock(id, quantity)?;
            } else {
                Prompt::remove_stock(storage)?;
            }
            Ok(true)
        }
        Restock {
            id,
            name,
            quantity,
            expiration_date,
        } => {
            match (id, name, expiration_date) {
                (Some(id), None, _) => storage.restock_product(id, quantity, expiration_date),
                (_, Some(name), _) => storage.restock_by_name(&name, quantity, expiration_date),
                _ => Prompt::restock_product(storage),
            }?;
            Ok(true)
        }
        List(list) => match list.cmd {
            ListSubcommands::Products(args) => {
                match (args.name, args.max_price, args.min_price, args.quality) {
                    (Some(name), _, _, _) => storage.search_product_name(&name),
                    (_, Some(max_price), _, _) => storage.list_with_max_price(max_price),
                    (_, _, Some(min_price), _) => storage.list_with_min_price(min_price),
                    (_, _, _, Some(quality)) => storage.list_with_quality(quality.to_lowercase()),
                    _ => storage.list_products(),
                }
                Ok(true)
            }
            ListSubcommands::Items(args) => {
                match (args.id, args.name, args.expired, args.expiring) {
                    (Some(id), None, None, None) => storage.list_items_with_id(id),
                    (Some(id), None, Some(true), None) => storage.list_expired_with_id(id),
                    (Some(id), None, None, Some(days)) => storage.list_expiring_with_id(id, days),
                    (_, Some(name), None, None) => storage.list_items_with_name(&name),
                    (_, Some(name), Some(true), None) => storage.list_expired_with_name(&name),
                    (_, Some(name), None, Some(days)) => storage.list_expiring_with_name(&name, days),
                    (None, None, Some(true), None) => storage.list_expired_items(),
                    (None, None, None, Some(days)) => storage.list_expiring_items(days),
                    (_, _, Some(_), Some(_)) => {
                        return Err(ReplError::base(ExpiredAndExpiring))
                    }
                    _ => storage.list_items(),
                }
                Ok(true)
            }
        },
        Load { file_path } => {
            match Storage::load(&file_path, storage) {
                Ok(_) => Ok(true),
                Err(e) => Err(e),
            }
        }

        CreateStorage => {
            match Prompt::storage_creation(storage) {
                Ok(_) => Ok(true),
                Err(e) => Err(e),
            }
        }

        Save { file_path } => {
            if let Some(file_path) = file_path {
                match storage.save_as(&file_path) {
                    Ok(_) => Ok(true),
                    Err(e) => Err(Box::new(e)),
                }
            } else {
                match storage.save() {
                    Ok(_) => Ok(true),
                    Err(_) => Err(ReplError::base(CouldNotSaveStorage)),
                }
            }
        }

        Exit => {
            match confirm_exit() {
                Ok(true) => Ok(false),
                Ok(false) => Ok(true),
                Err(e) => Err(e),
            }
        }

        ForceExit => Ok(false),
        _ => Ok(false),
    }
}

fn respond(line: &str, storage: &mut Storage) -> Result<bool, Box<dyn Error>> {
    let args = line.split_whitespace().map(|s| s.to_string()).collect::<Vec<String>>();
    let cli = Repl::try_parse_from(args)?;
    resolve_cmd(cli.cmd, storage)
}

fn confirm_exit() -> Result<bool, Box<dyn Error>> {
    println!("Are you sure you want to exit? (y/n)");
    let mut input = String::new();
    match stdin().read_line(&mut input) {
        Ok(_) => match input.trim() {
            "y" => Ok(true),
            "n" => Ok(false),
            _ => confirm_exit(),
        },
        Err(_) => confirm_exit(),
    }
}

fn run_repl(storage: &mut Storage) -> Result<(), Box<dyn Error>> {
    loop {
        let line = readline()?;
        match respond(&line, storage) {
            Ok(true) => continue,
            Ok(false) => break,
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

pub fn run(args: Cli) -> Result<(), Box<dyn Error>> {
    let mut storage = Storage::new("default".to_string(), None);

    if let Some(file_path) = &args.storage_path {
        let name = file_path.as_str();
        let default_path_name = format!("./storage-{}.json", name);
        let default_path = Path::new(&default_path_name);
        if default_path.exists() {
            storage.file_path = String::from(&default_path_name);
            match Storage::load(&default_path_name, &mut storage) {
                Ok(_) => {},
                Err(e) => return Err(e),
            }
        } else if Path::new(name).exists() {
            storage.file_path = name.to_string();
            match Storage::load(name, &mut storage) {
                Ok(_) => {},
                Err(e) => return Err(e),
            }
        } else {
            return Err(ReplError::base(CouldNotLoadStorage));
        } 

        if let Some(cmd) = args.cmd {
            use Commands::*;
            match cmd {
                Save { .. } => Err(ReplError::base(InteractiveModeOnly)),
                Load { .. } => Err(ReplError::base(InteractiveModeOnly)),
                Exit => Err(ReplError::base(InteractiveModeOnly)),
                ForceExit => Err(ReplError::base(InteractiveModeOnly)),
                _ => {
                    resolve_cmd(cmd, &mut storage)?;
                    Ok(())
                }
            }
        } else {
            run_repl(&mut storage)?;
            Ok(())
        }
    } else {
        let default_path = Path::new("./storage-default.json");
        if default_path.exists() {
            storage.file_path = String::from("./storage-default.json");
            match Storage::load("./storage-default.json", &mut storage) {
                Ok(_) => {},
                Err(e) => return Err(e),
            }
        }
        run_repl(&mut storage)?;
        Ok(())
    }
}
