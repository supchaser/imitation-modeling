use std::io::Write;
use crate::models::{Transaction, FEC, CEC};
use std::collections::BinaryHeap;

pub fn write_header(file: &mut std::fs::File) -> std::io::Result<()> {
    writeln!(file, "| {:<6} | {:<8} | {:<6} | {:<70} | {:<70} |", "Фаза", "Таймер", "Ресурс", "FEC", "CEC")
}

pub fn write_log(
    file: &mut std::fs::File,
    phase: &str,
    time: f64,
    busy_count: usize,
    fec_str: &str,
    cec_str: &str,
) -> std::io::Result<()> {
    writeln!(
        file,
        "| {:<6} | {:<8.1} | {:<6} | {:<70} | {:<70} |",
        phase,
        time,
        busy_count,
        fec_str,
        cec_str
    )
}

pub fn format_transaction(t: &Transaction) -> String {
    format!(
        "{},{:.1},{},{}",
        t.id,
        t.time,
        t.current_block as i32,
        t.next_block as i32
    )
}

pub fn format_list(items: &[String]) -> String {
    if items.is_empty() {
        return "\"\"".to_string();
    }
    let s = items.join("; ");
    format!("\"{}\"", s)
}

pub fn format_fec(fec: &FEC) -> String {
    if fec.is_empty() {
        return "\"\"".to_string();
    }
    let mut temp_heap: BinaryHeap<Transaction> = fec.heap.clone();
    let mut items = Vec::new();
    while let Some(t) = temp_heap.pop() {
        items.push(format_transaction(&t));
    }
    items.sort_by(|a, b| {
        let time_a = a.split(',').nth(1).unwrap().parse::<f64>().unwrap_or(0.0);
        let time_b = b.split(',').nth(1).unwrap().parse::<f64>().unwrap_or(0.0);
        time_a.partial_cmp(&time_b).unwrap_or(std::cmp::Ordering::Equal)
    });
    format_list(&items)
}

pub fn format_cec(cec: &CEC) -> String {
    if cec.is_empty() {
        return "\"\"".to_string();
    }
    let items: Vec<String> = cec.queue.iter().map(|t| format_transaction(t)).collect();
    format_list(&items)
}