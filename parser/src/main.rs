use serde::{Deserialize, Serialize};
use scraper::{Html, Selector, ElementRef};
use std::{fs, collections::HashSet, thread, time::Duration, cmp::Ordering};
use reqwest::Client;
use serde_json::json;
use html_escape::decode_html_entities;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
enum CardType {
    LEADER,
    STAGE,
    EVENT,
    CHARACTER,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
enum Rarity {
    #[serde(rename = "C")]
    Common,
    #[serde(rename = "UC")]
    Uncommon,
    #[serde(rename = "R")]
    Rare,
    #[serde(rename = "SR")]
    SuperRare,
    #[serde(rename = "L")]
    Leader,
    #[serde(rename = "SP CARD")]
    SpecialCard,
    #[serde(rename = "SEC")]
    SecretRare,
    #[serde(rename = "P")]
    Promo,
    #[serde(rename = "TR")]
    TreasureRare,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Color {
    Red,
    Blue,
    Green,
    Yellow,
    Black,
    Purple,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
enum Effect {
    #[serde(rename = "[Activate: Main]")]
    ActivateMain,
    #[serde(rename = "[Banish]")]
    Banish,
    #[serde(rename = "[Blocker]")]
    Blocker,
    #[serde(rename = "[Counter]")]
    Counter,
    #[serde(rename = "[DON!! x1]")]
    DonX1,
    #[serde(rename = "[DON!! x2]")]
    DonX2,
    #[serde(rename = "[Double Attack]")]
    DoubleAttack,
    #[serde(rename = "[End of Your Turn]")]
    EndOfYourTurn,
    #[serde(rename = "[Main]")]
    Main,
    #[serde(rename = "[On Block]")]
    OnBlock,
    #[serde(rename = "[On K.O.]")]
    OnKO,
    #[serde(rename = "[On Play]")]
    OnPlay,
    #[serde(rename = "[On Your Opponent's Attack]")]
    OnOpponentsAttack,
    #[serde(rename = "[Once Per Turn]")]
    OncePerTurn,
    #[serde(rename = "[Opponent's Turn]")]
    OpponentsTurn,
    #[serde(rename = "[Rush]")]
    Rush,
    #[serde(rename = "[Trigger]")]
    Trigger,
    #[serde(rename = "[When Attacking]")]
    WhenAttacking,
    #[serde(rename = "[Your Turn]")]
    YourTurn,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq)]
struct Card {
    card_name: String,
    card_number: String,
    rarity: Rarity,
    is_alternate_art: bool,
    card_type: CardType,
    image_url: String,
    life: String,
    cost: String,
    attributes: Vec<String>,
    power: String,
    counter: String,
    colors: Vec<Color>,
    types: Vec<String>,
    effects: Option<String>,
    card_effects: Vec<String>,
    card_sets: String,
    image_name: String,
}

impl PartialEq for Card {
    fn eq(&self, other: &Self) -> bool {
        self.card_sets == other.card_sets && self.card_number == other.card_number
    }
}

impl PartialOrd for Card {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Card {
    fn cmp(&self, other: &Self) -> Ordering {
        // First compare by card sets
        match self.card_sets.cmp(&other.card_sets) {
            Ordering::Equal => {
                // If card sets are equal, compare card numbers
                // Split the card number into prefix and numeric parts
                let (self_prefix, self_num) = split_card_number(&self.card_number);
                let (other_prefix, other_num) = split_card_number(&other.card_number);
                
                // First compare prefixes
                match self_prefix.cmp(&other_prefix) {
                    Ordering::Equal => {
                        // If prefixes are equal, compare numeric parts
                        self_num.cmp(&other_num)
                    },
                    ordering => ordering,
                }
            },
            ordering => ordering,
        }
    }
}

// Helper function to split card numbers like "ST01-001" into ("ST01-", 1)
fn split_card_number(card_number: &str) -> (String, i32) {
    let numeric_part_start = card_number
        .chars()
        .position(|c| c.is_ascii_digit())
        .unwrap_or(0);
    
    let (prefix, number_str) = card_number.split_at(numeric_part_start);
    
    // Find where the last numeric part begins
    let last_dash = number_str.rfind('-').unwrap_or(0);
    let final_number = if last_dash > 0 {
        &number_str[last_dash + 1..]
    } else {
        number_str
    };
    
    // Extract just the numbers, defaulting to 0 if parsing fails
    let number = final_number
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse::<i32>()
        .unwrap_or(0);
    
    // Construct the prefix including everything up to the final number
    let full_prefix = if last_dash > 0 {
        format!("{}{}-", prefix, &number_str[..last_dash])
    } else {
        prefix.to_string()
    };
    
    (full_prefix, number)
}

#[derive(Debug)]
struct CardSource {
    url: String,
    colors: Vec<&'static str>,
    region: &'static str,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sources = vec![
        CardSource {
            url: "https://en.onepiece-cardgame.com/cardlist/".to_string(),
            colors: vec!["Red", "Green", "Blue", "Purple", "Black", "Yellow"],
            region: "en",
        },
        CardSource {
            url: "https://asia-en.onepiece-cardgame.com/cardlist/".to_string(),
            colors: vec!["Red", "Green", "Blue", "Purple", "Black", "Yellow"],
            region: "jp",
        },
    ];

    let client = Client::new();
    fs::create_dir_all("input")?;

    for source in sources {
        for color in source.colors {
            println!("Fetching {} cards...", color);
            
            let form_data = [
                ("freewords", ""),
                ("series", ""),
                ("colors[]", color),
            ];

            let response = client
                .post(&source.url)
                .form(&form_data)
                .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:122.0) Gecko/20100101 Firefox/122.0")
                .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8")
                .header("Accept-Language", "en-GB,en;q=0.9")
                .header("Accept-Encoding", "gzip, deflate, br")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("DNT", "1")
                .header("Connection", "keep-alive")
                .header("Upgrade-Insecure-Requests", "1")
                .send()
                .await?;

            let html_content = response.text().await?;
            
            // Save the HTML file
            let filename = format!("input/cardlist-{}-{}.html", color.to_lowercase(), source.region);
            fs::write(&filename, &html_content)?;
            
            // Parse the cards
            let cards = parse_cards(&html_content, source.region, true)?;
            
            // Save the output
            save_output(&cards, source.region)?;
            
            // Be nice to the server
            thread::sleep(Duration::from_secs(2));
        }
    }

    Ok(())
}

fn parse_cards(html_content: &str, base_image_type: &str, merge: bool) -> Result<Vec<Card>, Box<dyn std::error::Error>> {
    let document = Html::parse_document(html_content);
    let modal_col_selector = Selector::parse("dl.modalCol").unwrap();
    
    let base_image_type = if base_image_type == "jp" {
        "".to_string()
    } else {
        format!("{}.", base_image_type)
    };

    let base_image_url = format!("https://{}onepiece-cardgame.com/images/cardlist/card/", base_image_type);
    
    let mut cards = if merge {
        load_existing_cards()?
    } else {
        Vec::new()
    };
    
    for element in document.select(&modal_col_selector) {
        let card = parse_single_card(&element, &base_image_url)?;
        
        if merge {
            if let Some(existing_idx) = find_existing_card(&cards, &card) {
                cards[existing_idx] = card;
            } else {
                cards.push(card);
            }
        } else {
            cards.push(card);
        }
    }
    
    Ok(cards)
}

fn parse_single_card(element: &ElementRef, base_image_url: &str) -> Result<Card, Box<dyn std::error::Error>> {
    let info_col_selector = Selector::parse(".infoCol > span").unwrap();
    let card_name_selector = Selector::parse(".cardName").unwrap();
    let front_col_selector = Selector::parse(".frontCol img").unwrap();
    let back_col_selector = Selector::parse(".backCol").unwrap();
    
    // Extract basic info
    let mut info_spans = element.select(&info_col_selector);
    let card_number = info_spans.next()
        .ok_or("Missing card number")?
        .text()
        .collect::<String>()
        .trim()
        .to_string();
        
    let rarity = parse_rarity(&info_spans.next()
        .ok_or("Missing rarity")?
        .text()
        .collect::<String>())?;
        
    let card_type = parse_card_type(&info_spans.next()
        .ok_or("Missing card type")?
        .text()
        .collect::<String>())?;
    
    // Extract card name
    let card_name = decode_html_entities(&element.select(&card_name_selector)
        .next()
        .ok_or("Missing card name")?
        .text()
        .collect::<String>()
        .replace(" (Parallel)", ""))
        .into_owned();
    
    // Extract image URL
    let image_src = element.select(&front_col_selector)
        .next()
        .ok_or("Missing image")?
        .value()
        .attr("data-src")
        .ok_or("Missing image src")?;
        
    let image_name = image_src.split('/').last()
        .ok_or("Invalid image URL")?
        .to_string();
        
    let image_url = format!("{}{}", base_image_url, image_name);
    
    // Extract back col info
    let back_col = element.select(&back_col_selector)
        .next()
        .ok_or("Missing back column")?;
    
    let (life, cost) = parse_life_cost(&back_col)?;
    let attributes = parse_attributes(&back_col)?;
    let power = parse_power(&back_col)?;
    let counter = parse_counter(&back_col)?;
    let colors = parse_colors(&back_col)?;
    let types = parse_types(&back_col)?;
    let (effects, card_effects) = parse_effects(&back_col)?;
    let card_sets = parse_card_sets(&back_col)?;
    
    let is_alternate_art = image_url.contains(&format!("{}_", card_number)) && 
        !card_sets.contains("Included in");
        
    let image_name = image_name.split('.').next()
        .ok_or("Invalid image name")?
        .to_string();
    
    Ok(Card {
        card_name,
        card_number,
        rarity,
        is_alternate_art,
        card_type,
        image_url,
        life,
        cost,
        attributes,
        power,
        counter,
        colors,
        types,
        effects: Some(effects),
        card_effects,
        card_sets,
        image_name,
    })
}

fn parse_life_cost(element: &ElementRef) -> Result<(String, String), Box<dyn std::error::Error>> {
    let cost_selector = Selector::parse(".cost").unwrap();
    if let Some(cost_element) = element.select(&cost_selector).next() {
        let content = cost_element.inner_html();
        if content.contains("Cost") {
            Ok(("-".to_string(), content.replace("<h3>Cost</h3>", "")))
        } else {
            Ok((content.replace("<h3>Life</h3>", ""), "-".to_string()))
        }
    } else {
        Ok(("-".to_string(), "-".to_string()))
    }
}

fn parse_attributes(element: &ElementRef) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let attribute_selector = Selector::parse(".attribute i").unwrap();
    let mut attributes = Vec::new();
    
    for attr in element.select(&attribute_selector) {
        let attr_text = decode_html_entities(&attr.text().collect::<String>()).into_owned();
        for part in attr_text.split('/') {
            attributes.push(part.trim().to_string());
        }
    }
    
    if attributes.is_empty() {
        attributes.push("-".to_string());
    }
    
    Ok(attributes)
}

fn parse_power(element: &ElementRef) -> Result<String, Box<dyn std::error::Error>> {
    let power_selector = Selector::parse(".power").unwrap();
    if let Some(power_element) = element.select(&power_selector).next() {
        Ok(power_element.inner_html().replace("<h3>Power</h3>", ""))
    } else {
        Ok("-".to_string())
    }
}

fn parse_counter(element: &ElementRef) -> Result<String, Box<dyn std::error::Error>> {
    let counter_selector = Selector::parse(".counter").unwrap();
    if let Some(counter_element) = element.select(&counter_selector).next() {
        Ok(counter_element.inner_html().replace("<h3>Counter</h3>", ""))
    } else {
        Ok("-".to_string())
    }
}

fn parse_colors(element: &ElementRef) -> Result<Vec<Color>, Box<dyn std::error::Error>> {
    let color_selector = Selector::parse(".color").unwrap();
    let mut colors = Vec::new();
    
    if let Some(color_element) = element.select(&color_selector).next() {
        let color_text = color_element.inner_html()
            .replace("<h3>Color</h3>", "");
            
        for color_str in color_text.split('/') {
            colors.push(match color_str.trim() {
                "Red" => Color::Red,
                "Blue" => Color::Blue,
                "Green" => Color::Green,
                "Yellow" => Color::Yellow,
                "Black" => Color::Black,
                "Purple" => Color::Purple,
                _ => return Err(format!("Unknown color: {}", color_str).into()),
            });
        }
    }
    
    Ok(colors)
}

fn parse_types(element: &ElementRef) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let type_selector = Selector::parse(".feature").unwrap();
    let mut types = Vec::new();
    
    if let Some(type_element) = element.select(&type_selector).next() {
        let type_text = type_element.inner_html()
            .replace("<h3>Type</h3>", "");
            
        for type_str in type_text.split('/') {
            let parsed_type = decode_html_entities(type_str.trim()).into_owned();
            types.push(match parsed_type.as_str() {
                "Smile" => "SMILE".to_string(),
                _ => parsed_type,
            });
        }
    }
    
    Ok(types)
}

fn parse_effects(element: &ElementRef) -> Result<(String, Vec<String>), Box<dyn std::error::Error>> {
    let text_selector = Selector::parse(".text").unwrap();
    let trigger_selector = Selector::parse(".trigger").unwrap();
    
    let mut effects = String::new();
    let mut card_effects = Vec::new();
    
    if let Some(text_element) = element.select(&text_selector).next() {
        effects = text_element.inner_html()
            .replace("<h3>Effect</h3>", "")
            .replace("</slash>", "")
            .replace("<slash>", "<Slash>");
    }
    
    if let Some(trigger_element) = element.select(&trigger_selector).next() {
        effects.push_str(" ");
        effects.push_str(&trigger_element.inner_html().replace("<h3>Trigger</h3>", ""));
    }
    
    // Parse card effects from the text
    for effect in Effect::iter() {
        if effects.contains(&effect.to_string()) {
            card_effects.push(effect.to_string());
        }
    }
    
    if card_effects.is_empty() {
        card_effects.push("-".to_string());
    }
    
    Ok((effects, card_effects))
}

fn parse_card_sets(element: &ElementRef) -> Result<String, Box<dyn std::error::Error>> {
    let set_selector = Selector::parse(".getInfo").unwrap();
    let mut card_sets = String::new();
    
    if let Some(set_element) = element.select(&set_selector).next() {
        card_sets = set_element.inner_html()
            .replace("<h3>Card Set(s)</h3>", "");
            
        // Handle special cases
        if card_sets == "OP-05" {
            card_sets = "[OP05] -AWAKENING OF THE NEW ERA- [OP05]".to_string();
        }
        
        card_sets = match card_sets.as_str() {
            "[OP-06] -Wings of Captain- [OP-06]" | "OP-06" => 
                "[OP-06] -WINGS OF THE CAPTAIN- [OP-06]".to_string(),
            "[OP-07] -500 Years in the Future- [OP-07]" | "OP-07" => 
                "[OP-07] -500 YEARS IN THE FUTURE- [OP-07]".to_string(),
            "[OP-08] -Two Legends- [OP-08]" | "OP-08" => 
                "[OP-08] -TWO LEGENDS- [OP-08]".to_string(),
            "[OP-09] -Emperors in the New World- [OP-09]" | "OP-09" => 
                "[OP-09] -EMPERORS IN THE NEW WORLD- [OP-09]".to_string(),
            "[EB-01] -Memorial Collection- [EB-01]" | "EB-01" => 
                "[EB-01] -MEMORIAL COLLECTION- [EB-01]".to_string(),
            _ => card_sets,
        };
        
        // Fix various formatting issues
        card_sets = card_sets
            .replace("[OP", "[OP-")
            .replace("[OP--", "[OP-")
            .replace("[EB", "[EB-")
            .replace("[EB--", "[EB-")
            .replace("[ST", "[ST-")
            .replace("[ST--", "[ST-")
            .replace("-[OP", "- [OP")
            .replace("-[EB", "- [EB")
            .replace("-[ST", "- [ST");
    }
    
    Ok(decode_html_entities(&card_sets).into_owned())
}

fn parse_rarity(text: &str) -> Result<Rarity, Box<dyn std::error::Error>> {
    Ok(match text.trim() {
        "C" => Rarity::Common,
        "UC" => Rarity::Uncommon,
        "R" => Rarity::Rare,
        "SR" => Rarity::SuperRare,
        "L" => Rarity::Leader,
        "SP CARD" => Rarity::SpecialCard,
        "SEC" => Rarity::SecretRare,
        "P" => Rarity::Promo,
        "TR" => Rarity::TreasureRare,
        _ => return Err(format!("Unknown rarity: {}", text).into()),
    })
}

fn parse_card_type(text: &str) -> Result<CardType, Box<dyn std::error::Error>> {
    Ok(match text.trim().to_uppercase().as_str() {
        "LEADER" => CardType::LEADER,
        "STAGE" => CardType::STAGE,
        "EVENT" => CardType::EVENT,
        "CHARACTER" => CardType::CHARACTER,
        _ => return Err(format!("Unknown card type: {}", text).into()),
    })
}

fn find_existing_card(cards: &[Card], new_card: &Card) -> Option<usize> {
    cards.iter()
        .position(|card| card.image_url == new_card.image_url)
}

fn load_existing_cards() -> Result<Vec<Card>, Box<dyn std::error::Error>> {
    if let Ok(content) = fs::read_to_string("../json/cards-full.json") {
        Ok(serde_json::from_str(&content)?)
    } else {
        Ok(Vec::new())
    }
}

fn save_output(cards: &[Card], region: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Create output directory if it doesn't exist
    let output_dir = format!("../json/{}", region);
    fs::create_dir_all(&output_dir)?;
    
    // Sort the cards
    let mut sorted_cards = cards.to_vec();
    sorted_cards.sort();
    
    // Save full cards data
    fs::write(
        format!("{}/cards-full.json", output_dir),
        serde_json::to_string_pretty(&sorted_cards)?,
    )?;
    
    // Save cards without effects
    let cards_without_effects: Vec<_> = sorted_cards.iter()
        .map(|card| {
            let mut card = card.clone();
            card.effects = None;
            card
        })
        .collect();
        
    fs::write(
        format!("{}/cards.json", output_dir),
        serde_json::to_string_pretty(&cards_without_effects)?,
    )?;
    
    // Generate and save filters
    let filters = generate_filters(&sorted_cards);
    fs::write(
        format!("{}/filters.json", output_dir),
        serde_json::to_string_pretty(&filters)?,
    )?;
    
    Ok(())
}

fn generate_filters(cards: &[Card]) -> serde_json::Value {
    let mut filters = serde_json::Map::new();
    
    // Collect unique values for each field
    let card_names: HashSet<_> = cards.iter().map(|c| &c.card_name).collect();
    let card_numbers: HashSet<_> = cards.iter().map(|c| &c.card_number).collect();
    let rarities: HashSet<_> = cards.iter().map(|c| &c.rarity).collect();
    let card_types: HashSet<_> = cards.iter().map(|c| &c.card_type).collect();
    let life_values: HashSet<_> = cards.iter().map(|c| &c.life).collect();
    let cost_values: HashSet<_> = cards.iter().map(|c| &c.cost).collect();
    let powers: HashSet<_> = cards.iter().map(|c| &c.power).collect();
    let counters: HashSet<_> = cards.iter().map(|c| &c.counter).collect();
    let card_sets: HashSet<_> = cards.iter().map(|c| &c.card_sets).collect();
    
    // Collect all unique attributes and types across all cards
    let mut attributes = HashSet::new();
    let mut types = HashSet::new();
    let mut card_effects = HashSet::new();
    
    for card in cards {
        attributes.extend(card.attributes.iter().cloned());
        types.extend(card.types.iter().cloned());
        card_effects.extend(card.card_effects.iter().cloned());
    }
    
    // Add all collected values to filters, sorted alphabetically
    filters.insert("card_names".to_string(), json!(sorted_vec(card_names)));
    filters.insert("card_numbers".to_string(), json!(sorted_vec(card_numbers)));
    filters.insert("rarities".to_string(), json!(sorted_vec(rarities)));
    filters.insert("card_types".to_string(), json!(sorted_vec(card_types)));
    filters.insert("life_values".to_string(), json!(sorted_vec(life_values)));
    filters.insert("cost_values".to_string(), json!(sorted_vec(cost_values)));
    filters.insert("powers".to_string(), json!(sorted_vec(powers)));
    filters.insert("counters".to_string(), json!(sorted_vec(counters)));
    filters.insert("attributes".to_string(), json!(sorted_vec(attributes)));
    filters.insert("types".to_string(), json!(sorted_vec(types)));
    filters.insert("card_effects".to_string(), json!(sorted_vec(card_effects)));
    filters.insert("card_sets".to_string(), json!(sorted_vec(card_sets)));
    
    serde_json::Value::Object(filters)
}

fn sorted_vec<T: Ord>(set: HashSet<T>) -> Vec<T> {
    let mut vec: Vec<T> = set.into_iter().collect();
    vec.sort();
    vec
}

impl Effect {
    fn iter() -> impl Iterator<Item = String> {
        use Effect::*;
        vec![
            ActivateMain, Banish, Blocker, Counter, DonX1, DonX2,
            DoubleAttack, EndOfYourTurn, Main, OnBlock, OnKO, OnPlay,
            OnOpponentsAttack, OncePerTurn, OpponentsTurn, Rush,
            Trigger, WhenAttacking, YourTurn
        ].into_iter().map(|e| e.to_string())
    }
    
    fn to_string(&self) -> String {
        match self {
            Effect::ActivateMain => "[Activate: Main]".to_string(),
            Effect::Banish => "[Banish]".to_string(),
            Effect::Blocker => "[Blocker]".to_string(),
            Effect::Counter => "[Counter]".to_string(),
            Effect::DonX1 => "[DON!! x1]".to_string(),
            Effect::DonX2 => "[DON!! x2]".to_string(),
            Effect::DoubleAttack => "[Double Attack]".to_string(),
            Effect::EndOfYourTurn => "[End of Your Turn]".to_string(),
            Effect::Main => "[Main]".to_string(),
            Effect::OnBlock => "[On Block]".to_string(),
            Effect::OnKO => "[On K.O.]".to_string(),
            Effect::OnPlay => "[On Play]".to_string(),
            Effect::OnOpponentsAttack => "[On Your Opponent's Attack]".to_string(),
            Effect::OncePerTurn => "[Once Per Turn]".to_string(),
            Effect::OpponentsTurn => "[Opponent's Turn]".to_string(),
            Effect::Rush => "[Rush]".to_string(),
            Effect::Trigger => "[Trigger]".to_string(),
            Effect::WhenAttacking => "[When Attacking]".to_string(),
            Effect::YourTurn => "[Your Turn]".to_string(),
        }
    }
}