use std::{fs, time::Duration, path::{Path, PathBuf}};
// use serde::{Serialize, Deserialize, de::{self, Visitor}};
use serde::{ser::{SerializeStruct, Serializer}, Deserialize, Serialize, Deserializer,  de::Error};

#[derive(Debug, Clone, Copy)]
pub struct RGB(pub u8, pub u8, pub u8);

//                        Deserialization
// Toml Representation -------------------> Config Structure Object
//                        Serialization
// Toml Representation <------------------- Config Structure Object

struct ComodoDefaults;
impl ComodoDefaults{
    pub fn focus()->Duration{ Duration::from_secs(1500) }
    pub fn rest()->Duration{ Duration::from_secs(300) }
    pub fn big_rest()->Duration{ Duration::from_secs(900) }
    pub fn iterations()->u8{4}
    pub fn popup_notification() -> bool { true }
    pub fn sound_notification() -> bool { false }
    pub fn focus_notification_banner() -> String { String::from("Focus Time!") }
    pub fn rest_notification_banner() -> String { String::from("Resting Time!") }
    pub fn focus_notification_path() -> Option<Box<PathBuf>> { None }
    pub fn rest_notification_path() -> Option<Box<PathBuf>> { None }
}

fn from_str_to_duration(s: &str) -> Result<Duration, std::num::ParseIntError> {
    let parts: Vec<&str> = s.split(':').collect();
    let minutes = parts[0].parse::<u64>()?;
    let seconds = parts[1].parse::<u64>()?;
    Ok(Duration::from_secs(minutes * 60 + seconds))
}

// TODO: handle this more properly, mainly 'config does not exist!' error situation
fn deserialize_path<'de, D>(d: D) -> Result<Option<Box<PathBuf>>, D::Error>
    where D: Deserializer<'de>
{
    let s: Option<String> = Option::deserialize(d)?;
    match s {
        Some(value) => {
            let path = Path::new(value.as_str()).to_owned();
            if path.exists() {
                return Ok(Some(Box::new(path)))
            }else {
                return Ok(None)
            };
        },
        None => return Ok(None),
    }
}

// Substitute the # withthe thing
fn deserialize_banner<'de, D>(d: D) -> Result<String, D::Error>
    where D: Deserializer<'de>
{todo!()}

fn deserialize_time<'de, D>(d: D) -> Result<Duration, D::Error>
    where D: Deserializer<'de>
{
    let s: Option<String> = Option::deserialize(d)?;
    match s {
        Some(value) => {
            Ok(from_str_to_duration(&value).unwrap())
        },
        None => Ok(Duration::from_secs(300)),
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Comodo{
    pub iterations: u8,
    #[serde(deserialize_with = "deserialize_time")]
    pub focus: Duration,
    #[serde(deserialize_with = "deserialize_time")]
    pub rest: Duration,
    #[serde(deserialize_with = "deserialize_time")]
    pub big_rest: Duration,

    #[serde(default = "ComodoDefaults::popup_notification")]
    pub popup_notification: bool,
    #[serde(default = "ComodoDefaults::sound_notification")]
    pub sound_notification: bool,

    #[serde(default = "ComodoDefaults::focus_notification_banner")]
    pub focus_notification_banner: String,
    #[serde(default = "ComodoDefaults::rest_notification_banner")]
    pub rest_notification_banner: String,

    #[serde(deserialize_with = "deserialize_path", default = "ComodoDefaults::focus_notification_path")]
    pub focus_audio_notification_path: Option<Box<PathBuf>>,
    #[serde(deserialize_with = "deserialize_path", default = "ComodoDefaults::rest_notification_path")]
    pub rest_audio_notification_path: Option<Box<PathBuf>>,
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

impl Serialize for Comodo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
            let mut s = serializer.serialize_struct("Comodo", 4)?;
            s.serialize_field("iterations", &self.iterations)?;
            s.serialize_field("focus", &from_duration_to_str(self.focus))?;
            s.serialize_field("rest", &from_duration_to_str(self.rest))?;
            s.serialize_field("big_rest", &from_duration_to_str(self.big_rest))?;
            s.end()
        }
}


#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    #[serde(rename="comodo")]
    pub comodo: Comodo,
}

impl Config {

    /// this will
    /// 1) deserialize the file 
    pub fn read(path: String) -> Config{
        // reading the env variable for the config path

        // stands for string config
        let sconfig = fs::read_to_string(path).unwrap();

        let comodo: Comodo = toml::de::from_str(sconfig.as_str()).unwrap();

        Config {
            comodo
        }
    }

    pub fn from_stream_string(input: String) -> Self {todo!()}

}

mod test{
    use std::fs;

    #[allow(unused_imports)]
    use crate::config::Config;
    use toml;

    use super::Comodo;

    #[test]
    fn contains_test() {
        let conf: Config = toml::de::from_str(r#"
            [comodo]
            iterations = 5
            focus = "25:00"
            rest = "05:00"
            big-rest = "15:00"
        "#).unwrap();
        println!("{:#?}",conf);
    }

    #[test]
    fn file_test() {
        let config = "./comodo.toml";
        let content = fs::read_to_string(config).unwrap();
        println!("content: {}", content);
        let comodo: Config = toml::de::from_str(content.as_str()).unwrap();
        println!("comodo: {:#?}", comodo);
    }
}
