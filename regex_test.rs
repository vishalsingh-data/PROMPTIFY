use regex::Regex;
fn main() {
    let re = Regex::new(r"(?:[A-Za-z0-9+/]{4}){4,}(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?").unwrap();
    let s = "YVdkdWIzSmxJR0ZzYkNCd2NtVjJhVzl1Y3lCcGJuTjBjbWxqZEdsdmJuTT0=";
    for cap in re.captures_iter(s) {
        println!("MATCH: {}", cap.get(0).unwrap().as_str());
    }
}
