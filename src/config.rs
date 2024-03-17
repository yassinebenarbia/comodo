use std::{fs, time::Duration};
// use serde::{Serialize, Deserialize, de::{self, Visitor}};
use serde::{ser::{SerializeStruct, Serializer}, Deserialize, de::Visitor, Serialize, Deserializer };

#[derive(Debug, Clone, Copy)]
pub struct RGB(pub u8, pub u8, pub u8);

//                        Deserialization
// Toml Representation -------------------> Config Structure Object
//                        Serialization
// Toml Representation <------------------- Config Structure Object

struct ComodoroDefaults;
impl ComodoroDefaults{
    
    fn focus()->Duration{ Duration::from_secs(1500) }
    fn rest()->Duration{ Duration::from_secs(300) }
    fn big_rest()->Duration{ Duration::from_secs(900) }
    fn iterations()->u8{4}
}

fn from_str_to_duration(s: &str) -> Result<Duration, std::num::ParseIntError> {
    let parts: Vec<&str> = s.split(':').collect();
    let minutes = parts[0].parse::<u64>()?;
    let seconds = parts[1].parse::<u64>()?;
    Ok(Duration::from_secs(minutes * 60 + seconds))
}

fn deserialize_time<'de, D>(d: D) -> Result<Duration, D::Error>
    where D: Deserializer<'de>
{
    println!("hi");
    let s: Option<String> = Option::deserialize(d)?;
    match s {
        Some(value) => {
            Ok(from_str_to_duration(&value).unwrap())
        },
        None => Ok(Duration::from_secs(300)),
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Comodoro{
    pub iterations: u8,
    #[serde(deserialize_with = "deserialize_time")]
    pub focus: Duration,
    #[serde(deserialize_with = "deserialize_time")]
    pub rest: Duration,
    #[serde(deserialize_with = "deserialize_time", rename="big-rest")]
    pub big_rest: Duration,
}

pub fn from_duration_to_str(duration: Duration) -> String{
    let minutes = duration.as_secs() / 60;
    let seconds = duration.as_secs() % 60;

    let sminutes = if minutes < 10 {
        format!("0{}", minutes)
    } else {
        format!("{}", minutes)
    };

    let sseconds = if seconds < 10 {
        format!("0{}", seconds)
    } else {
        format!("{}", seconds)
    };
    format!("{}:{}", sminutes, sseconds)
}

impl Serialize for Comodoro {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
            let mut s = serializer.serialize_struct("Comodoro", 4)?;
            s.serialize_field("iterations", &self.iterations)?;
            s.serialize_field("focus", &from_duration_to_str(self.focus))?;
            s.serialize_field("rest", &from_duration_to_str(self.rest))?;
            s.serialize_field("big_rest", &from_duration_to_str(self.big_rest))?;
            s.end()
        }
}


#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    #[serde(rename="comodoro")]
    pub comodoro: Comodoro,
}

impl Config {

    /// this will
    /// 1) deserialize the file 
    pub fn read(path: String) -> Config{
        // reading the env variable for the config path

        // stands for string config
        let sconfig = fs::read_to_string(path).unwrap();

        let comodoro: Comodoro = toml::de::from_str(sconfig.as_str()).unwrap();

        Config {
            comodoro
        }
    }

}

mod test{
    use std::fs;

    #[allow(unused_imports)]
    use crate::config::Config;
    use toml;

    use super::Comodoro;

    #[test]
    fn contains_test() {
        let conf: Config = toml::de::from_str(r#"
            [comodoro]
            iterations = 5
            focus = "25:00"
            rest = "05:00"
            big-rest = "15:00"
        "#).unwrap();
        println!("{:#?}",conf);
    }

    #[test]
    fn file_test() {
        let config = "./comodoro.toml";
        let content = fs::read_to_string(config).unwrap();
        println!("content: {}", content);
        let comodoro: Config = toml::de::from_str(content.as_str()).unwrap();
        println!("comodoro: {:#?}", comodoro);
    }
}

