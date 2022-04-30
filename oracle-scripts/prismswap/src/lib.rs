use obi::{OBIDecode, OBIEncode, OBISchema};
use owasm_kit::{execute_entry_point, ext, oei, prepare_entry_point};
use std::collections::hash_map::*;
use std::collections::HashMap;
use std::str::FromStr;
use strum::{EnumProperty, IntoEnumIterator};
use strum_macros::{Display, EnumIter, EnumProperty as EnumPropertyTrait, EnumString};

#[derive(OBIDecode, OBISchema)]
struct Input {
    symbols: Vec<String>,
    multiplier: u64,
}

#[derive(OBIEncode, OBISchema)]
struct Output {
    rates: Vec<u64>,
}

const EXCHANGE_COUNT: u64 = 1;

#[derive(Display, EnumString, EnumIter, PartialEq, Debug, Copy, Clone)]
#[allow(clippy::upper_case_acronyms)]
enum Token {
    PRISM,
    CLUNA,
    PLUNA,
    YLUNA,
}

#[derive(Display, EnumString, EnumIter, EnumPropertyTrait, Debug, Copy, Clone, PartialEq)]
enum Exchange {
    #[strum(props(data_source_id = "298"))]
    Astroport = 0,
}

impl Exchange {
    fn from_u64(value: u64) -> Option<Exchange> {
        Exchange::iter().nth(value as usize)
    }
}

macro_rules! token_to_exchange_list {
    ($data:expr) => {
        match $data {
            Token::PRISM => "1",
            Token::CLUNA => "1",
            Token::PLUNA => "1",
            Token::YLUNA => "1",
        }
    };
}

fn get_ds_from_exchange(exchange_id: u64) -> i64 {
    let exchange = match Exchange::from_u64(exchange_id) {
        Some(data) => data,
        None => panic!("Unsupported Exchange ID"),
    };
    i64::from_str(exchange.get_str("data_source_id").unwrap()).unwrap()
}

// Get list of exchange that needs to be called along with the tokens to call
// given a list of input tokens
fn get_exchange_map(symbols: Vec<String>) -> HashMap<u64, Vec<Token>> {
    let mut exchange_map = HashMap::new();
    for symbol in symbols {
        let symbol_token = Token::from_str(symbol.as_str()).unwrap();
        let mut exchange_binary = token_to_exchange_list!(symbol_token).chars();
        for i in 0..(EXCHANGE_COUNT as usize) {
            if exchange_binary.next() == Some('1') {
                match exchange_map.entry(i as u64) {
                    Entry::Vacant(e) => {
                        e.insert(vec![symbol_token]);
                    }
                    Entry::Occupied(mut e) => {
                        e.get_mut().push(symbol_token);
                    }
                }
            }
        }
    }
    exchange_map
}

fn median(arr: &mut Vec<f64>) -> f64 {
    let len_arr = arr.len() as f64;
    if len_arr > 0f64 {
        arr.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = len_arr / 2f64;
        if len_arr as u64 % 2 == 0 {
            (arr[(mid - 1f64) as usize] + arr[mid as usize]) / 2f64
        } else {
            arr[mid as usize]
        }
    } else {
        0f64
    }
}

#[no_mangle]
fn prepare_impl(input: Input) {
    let exchange_map = get_exchange_map(input.symbols);
    for (exchange_id, symbols) in exchange_map.iter() {
        oei::ask_external_data(
            *exchange_id as i64,
            get_ds_from_exchange(*exchange_id),
            symbols
                .iter()
                .map(|&x| x.to_string())
                .collect::<Vec<_>>()
                .join(" ")
                .to_string()
                .as_bytes(),
        )
    }
}

#[no_mangle]
fn execute_impl(input: Input) -> Output {
    // Get the required exchange and associated tokens to query
    let exchange_map = get_exchange_map((*input.symbols).to_vec());
    // store the median price of each token requested from an exchange
    let mut exchange_medians: Vec<Option<Vec<f64>>> = vec![Some(vec![]); EXCHANGE_COUNT as usize];
    for (exchange_id, _symbols) in exchange_map.iter() {
        // Get the data source calldata for a given external ID
        let raw_input = ext::load_input::<String>(*exchange_id as i64);
        let mut prices = vec![vec![]; exchange_map[exchange_id].len()];
        let inputs: Vec<String> = raw_input.collect();
        if inputs.is_empty() {
            exchange_medians[*exchange_id as usize] = None;
            continue;
        }
        // for each validator response for the exchange,
        // split the response into individual prices
        for raw in inputs {
            let px_list: Vec<f64> = raw
                .split(',')
                .filter_map(|x| x.parse::<f64>().ok())
                .collect();
            // for each token price, add it to the list of validator responses
            // for that token and exchange
            for (idx, &px) in px_list.iter().enumerate() {
                prices[idx].push(px);
            }
        }
        let mut median_prices = vec![0f64; prices.len()];
        for (idx, price) in prices.iter().enumerate() {
            median_prices[idx] = median(&mut price.to_vec());
        }
        exchange_medians[*exchange_id as usize] = Some(median_prices);
    }

    let mut symbol_pxs = HashMap::new();
    for (exchange_id, symbols) in exchange_map.iter() {
        let exchange_median = exchange_medians[*exchange_id as usize].as_ref();
        if exchange_median.is_none() {
            continue;
        }
        let exchange_median = exchange_median.unwrap();
        let symbols_vec: Vec<String> = symbols.iter().map(|x| x.to_string()).collect();

        for (symbol_id, symbol) in symbols_vec.iter().enumerate() {
            match symbol_pxs.entry(symbol.clone()) {
                Entry::Vacant(e) => {
                    e.insert(vec![exchange_median[symbol_id]]);
                }
                Entry::Occupied(mut e) => {
                    e.get_mut().push(exchange_median[symbol_id]);
                }
            }
        }
    }

    let mut rates = Vec::new();
    for symbol in input.symbols.iter() {
        rates.push((median(symbol_pxs.get_mut(symbol).unwrap()) * (input.multiplier as f64)) as u64)
    }
    Output { rates }
}

prepare_entry_point!(prepare_impl);
execute_entry_point!(execute_impl);
