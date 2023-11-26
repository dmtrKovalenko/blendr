use clap::Parser;
use std::collections::HashMap;

fn parse_name_map(path: &str) -> Result<HashMap<uuid::Uuid, String>, clap::Error> {
    let path = std::path::Path::new(path);

    if !path.exists() || !path.is_file() {
        return Err(clap::Error::raw(
            clap::error::ErrorKind::InvalidValue,
            format!("File {} does not exist.", path.display()),
        ));
    }

    let content = std::fs::read_to_string(path)?;

    content
        .trim()
        .split('\n')
        .enumerate()
        .map(|(i, line)| -> clap::error::Result<_> {
            let [uuid, name]: [&str; 2] =
                line.split('=')
                    .collect::<Vec<_>>()
                    .try_into()
                    .map_err(|_| {
                        clap::Error::raw(
                            clap::error::ErrorKind::InvalidValue,
                            format!("Failed to parse line {i} from file {}: Missing desired structure UUID=NAME.\nNames map file supports very simple key-value pairs format where first value is uuid of service or characteristic and the second is the name.\n\ne.g. 0000FFE0-0000-1000-8000-00805F9B34FB=Cpu Tempreture", path.display()),
                        )
                    })?;

            let uuid = uuid::Uuid::parse_str(uuid).map_err(|e| {
                clap::Error::raw(
                    clap::error::ErrorKind::InvalidValue,
                    format!(
                        "Failed to parse uuid on the left side of the line {i} from file {}: {e}",
                        path.display()
                    ),
                )
            })?;
            Ok((uuid, name.trim().to_owned()))
        })
        .collect::<clap::error::Result<HashMap<uuid::Uuid, String>>>()
}

#[test]
fn test_parse_name_map() {
    let test_path = std::path::Path::new("test.ini");
    std::fs::write(test_path, "0000FFE0-0000-1000-8000-00805F9B34FB=test data")
        .expect("Unable to write file");

    assert_eq!(
        parse_name_map(
            test_path
                .to_str()
                .expect("Unable to locate path of test .ini file")
        )
        .unwrap(),
        HashMap::from([(
            uuid::Uuid::from_u128(0x0000FFE0_0000_1000_8000_00805F9B34FB),
            "test data".to_string()
        )])
    );

    std::fs::remove_file(test_path).expect("Unable to delete file");
}

#[derive(Default, PartialEq, Eq, Debug, Clone, Copy, clap::ValueEnum)]
pub enum GeneralSort {
    Name,
    #[default]
    /// The default sort based on the trait implementer.
    DefaultSort,
}

pub trait GeneralSortable {
    fn cmp(&self, sort: &GeneralSort, a: &Self, b: &Self) -> std::cmp::Ordering;
}

impl GeneralSort {
    #[allow(dead_code)] // ? It is actually used in the code, some clippy bug
    pub fn apply_sort<T: GeneralSortable>(&self, a: &T, b: &T) -> std::cmp::Ordering {
        a.cmp(self, a, b)
    }
}

#[derive(Debug, Parser)]
#[command(
    version=env!("CARGO_PKG_VERSION"),
    author = "Dmitriy Kovalenko <dmtr.kovalenko@outlook.com>", 
    about="vim-style BLE browser terminal client",
    long_about="Blendr is a BLE browser terminal library. It allows to search for BLE peripherals, establish connections, interact with their services and characteristics, and read and write data right from your terminal."
)]
pub struct Args {
    #[clap(long, short)]
    /// Bluetooth adapter hardware index to use, if many available.
    /// If not specified, the first discovered adapter will be used.
    #[clap(default_value_t = 0)]
    pub adapter_index: usize,

    #[clap(long, short = 'i')]
    /// Scan interval in milliseconds.
    #[clap(default_value_t = 1000)]
    pub scan_interval: u64,

    #[clap(long, short)]
    #[clap(default_value_t = String::from("(?i)"))]
    /// Regex flags that by default applier to the filter queries.
    /// By default contains case-insensitive flag (?i). Pass --regex-flags "" to make searches case sensitive.
    pub regex_flags: String,

    /// Device name to search for on start. If only one device would be found matching this filter it will be connected automatically.
    #[clap(short, long)]
    pub device: Option<String>,

    /// Characteristic or service uui search that will be applied on start. If one characteristic will be found matching this filter it will be selected automatically.
    #[clap(short, long)]
    pub characteristic: Option<String>,

    /// Customize displaying of names and services
    /// Path to file in .ini like format (with no support of [Groups]) where keys are uuids of services or characteristics and values are names to display.
    ///
    /// # Example
    ///
    /// ```
    /// 0000FFE0-0000-1000-8000-00805F9B34FB=Cpu Tempreture
    /// 4f25b5f6-01d9-4d95-86a4-81e3d2f13b8f=My Custom Service data
    /// ```
    #[clap(long, value_parser = clap::builder::ValueParser::new(parse_name_map))]
    pub names_map_file: Option<HashMap<uuid::Uuid, String>>,

    /// Default sort type for all the views and lists.
    #[clap(long)]
    #[arg(value_enum)]
    pub sort: Option<GeneralSort>,
}
