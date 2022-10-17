use chrono::{TimeZone, Utc};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::thread;
use std::time::Duration;
use std::{io, path::Path};

#[derive(Serialize, Deserialize)]
pub struct Message {
    index: u64,
    sender: String,
    date: f64,
    msg: String,
}

#[derive(Serialize, Deserialize)]
pub struct JsonOutput {
    index: u64,
    sender: String,
    date: f64,
    msg: String,
}

#[derive(Serialize, Deserialize)]
pub struct CSVOutput  {
    index: u64,
    sender: String,
    date: f64,
    msg: String,
}

#[derive(Serialize, Deserialize)]
pub struct SyslogOutput  {
    index: u64,
    sender: String,
    date: f64,
    msg: String,
}

#[derive(Serialize, Deserialize)]
struct MessageLast {
    index: usize,
}

pub fn build_reqwest_client(cert_file: impl AsRef<Path>) -> anyhow::Result<Client> {
    // read a local binary DER encoded certificate
    let pem = std::fs::read(cert_file)?;

    // create a certificate
    let cert = reqwest::Certificate::from_pem(&pem)?;

    let client = reqwest::blocking::Client::builder()
        .add_root_certificate(cert)
        .build()?;

    Ok(client)
}

pub fn enter_tchat(
    client: &reqwest::blocking::Client,
    (username, password): (&str, &str),
) -> anyhow::Result<()> {
    let response = client
        .get("https://mychat.com:40443/enter")
        .basic_auth(username, Some(password))
        .send()?;

    if response.status() == reqwest::StatusCode::NOT_ACCEPTABLE {
        println!("Tu es déjà dans la room")
    }
    Ok(())
}

pub fn send_message(
    client: &reqwest::blocking::Client,
    (username, password): (&str, &str),
    msg: impl Into<String>,
) -> anyhow::Result<()> {
    let my_message = json!([msg.into(),]);

    let response = client
        .post("https://mychat.com:40443/send")
        .basic_auth(username, Some(password))
        .json(&my_message)
        .send()?;
    if response.status() == reqwest::StatusCode::NOT_ACCEPTABLE {
        println!("Erreur status code")
    }
    Ok(())
}

pub fn get_messages(
    client: &reqwest::blocking::Client,
    (username, password): (&str, &str),
    index: usize,
    len: usize,
    mut callback: impl FnMut(Message) -> anyhow::Result<()>,
) -> anyhow::Result<usize> {
    let mut nb_mess = 0;

    let mut tmp = Message {
        date: 0.0,
        index: 0,
        msg: String::new(),
        sender: String::new(),
    };

    // requête de la forme "https://mychat.com:40443/get?index={}&len={}"
    let response = client
        .get("https://mychat.com:40443/get")
        .query(&[("index", index), ("len", len)])
        .basic_auth(username, Some(password))
        .send()?;

    let texte = response.text()?;

    let test_json: Vec<Message> = serde_json::from_str(&texte)?;

    for elt in test_json {
        match tmp.write_msg(&elt) {
            Ok(_) => {}
            Err(error) => return Err(error),
        };

        // Appel de la fonction callback. Si le traitement sur le message reçu se passe bien
        // on incrémente le compteur. Sinon on aura une erreur
        match callback(elt) {
            Ok(_) => nb_mess += 1,
            Err(_) => {
                println!("Erreur de l'appel de la callback")
            }
        };

        match tmp.flush() {
            Ok(_) => {}
            Err(error) => return Err(error),
        };
    }

    // On retourne le nombre de message qui ont été traité
    Ok(nb_mess)
}

pub fn get_last(
    client: &reqwest::blocking::Client,
    (username, password): (&str, &str),
) -> anyhow::Result<usize> {
    let response = client
        .get("https://mychat.com:40443/last")
        .basic_auth(username, Some(password))
        .send()?;

    if response.status() == reqwest::StatusCode::NOT_ACCEPTABLE {
        println!("Erreur pour get_last")
    }

    let texte = response.text()?;

    let test_json: MessageLast = serde_json::from_str(&texte)?;

    Ok(test_json.index)
}

pub fn leave_tchat(
    client: &reqwest::blocking::Client,
    (username, password): (&str, &str),
) -> anyhow::Result<()> {
    let response = client
        .get("https://mychat.com:40443/leave")
        .basic_auth(username, Some(password))
        .send()?;
    if response.status() == reqwest::StatusCode::NOT_ACCEPTABLE {
        println!("Tu es déjà parti de la room");
    }
    Ok(())
}

pub trait MsgOutput {
    fn write_msg(&mut self, msg: &Message) -> anyhow::Result<usize>;
    fn flush(&mut self) -> anyhow::Result<()>;
    // fn JsonOutput(&mut self, msg: &Message) -> anyhow::Result<()>;
    // fn CSVOutput(&mut self, msg: &Message) -> anyhow::Result<()>;
    // fn SyslogOutput(&mut self, msg: &Message) -> anyhow::Result<()>;

}

impl MsgOutput for Message {

    fn write_msg(&mut self, msg: &Message) -> anyhow::Result<usize> {
        let path = "tchat_out.txt";

        let already = Path::new(path).exists();
        if !already {
            match File::create(path) {
                Ok(_) => {}
                Err(error) => println!("Erreur de création de fichier : {}", error),
            }
        }

        match OpenOptions::new().write(true).append(true).open(path) {
            Ok(mut file) => {
                let offset_utc_hour = 2.0;
                let offset_utc_sec: f64 = offset_utc_hour * 3600.0;
                let mut message = String::new();
                let date_updated = Utc.timestamp((msg.date + offset_utc_sec) as i64, 0);
                message = message + &date_updated.to_string() + "+" + &offset_utc_hour.to_string();
                message = message + " : (" + &msg.sender + ")";
                message = message + " > " + &msg.msg;

                writeln!(file, "{}", message)?;
            }
            Err(_) => println!("Erreur de création de fichier"),
        };

        Ok(1)
    }

    fn flush(&mut self) -> anyhow::Result<()> {
        // return pour que la fonction ne soit pas warningsée
        match io::Write::flush(&mut io::stdout()) {
            Ok(_) => {}
            Err(error) => println!("Erreur du flush : {}", error),
        };
        Ok(())
    }

}

impl MsgOutput for JsonOutput {
    fn write_msg(&mut self, msg: &Message) -> anyhow::Result<usize> {
        
        let path = "json_output.txt";

        let already = Path::new(path).exists();
        if !already {
            match File::create(path) {
                Ok(_) => {}
                Err(error) => println!("Erreur de création de fichier : {}", error),
            }
        }
        match OpenOptions::new().write(true).append(true).open(path) {
            Ok(mut file) => {
                let offset_utc_hour = 2.0;
                let offset_utc_sec: f64 = offset_utc_hour * 3600.0;
                let mut message = String::new();
                let date_updated = Utc.timestamp((msg.date + offset_utc_sec) as i64, 0);
                message = message + &date_updated.to_string() + "+" + &offset_utc_hour.to_string();
                message = message + " : (" + &msg.sender + ")";
                message = message + " > " + &msg.msg;

                writeln!(file, "{}", message)?;
            }
            Err(_) => println!("Erreur de création de fichier"),
        };
        Ok(1)
    }

    fn flush(&mut self) -> anyhow::Result<()> {
        // return pour que la fonction ne soit pas warningsée
        match io::Write::flush(&mut io::stdout()) {
            Ok(_) => {}
            Err(error) => println!("Erreur du flush : {}", error),
        };
        Ok(())
    }
}

impl MsgOutput for CSVOutput {
    fn write_msg(&mut self, msg: &Message) -> anyhow::Result<usize> {
        let path = "csv_output.txt";
        let already = Path::new(path).exists();
        if !already {
            match File::create(path) {
                Ok(_) => {}
                Err(error) => println!("Erreur de création de fichier : {}", error),
            }
        }
        match OpenOptions::new().write(true).append(true).open(path) {
            Ok(mut file) => {
                let offset_utc_hour = 2.0;
                let offset_utc_sec: f64 = offset_utc_hour * 3600.0;
                let mut message = String::new();
                let date_updated = Utc.timestamp((msg.date + offset_utc_sec) as i64, 0);
                message = message + &date_updated.to_string() + "+" + &offset_utc_hour.to_string();
                message = message + " : (" + &msg.sender + ")";
                message = message + " > " + &msg.msg;
                writeln!(file, "{}", message)?;
            }
            Err(_) => println!("Erreur de création de fichier"),
        };
        Ok(1)
    }

    fn flush(&mut self) -> anyhow::Result<()> {
        // return pour que la fonction ne soit pas warningsée
        match io::Write::flush(&mut io::stdout()) {
            Ok(_) => {}
            Err(error) => println!("Erreur du flush : {}", error),
        };
        Ok(())
    }
}

impl MsgOutput for SyslogOutput {
    fn write_msg(&mut self, msg: &Message) -> anyhow::Result<usize> {
        let path = "syslog_output.txt";

        let already = Path::new(path).exists();
        if !already {
            match File::create(path) {
                Ok(_) => {}
                Err(error) => println!("Erreur de création de fichier : {}", error),
            }
        }

        match OpenOptions::new().write(true).append(true).open(path) {
            Ok(mut file) => {
                let offset_utc_hour = 2.0;
                let offset_utc_sec: f64 = offset_utc_hour * 3600.0;
                let mut message = String::new();
                let date_updated = Utc.timestamp((msg.date + offset_utc_sec) as i64, 0);
                message = message + &date_updated.to_string() + "+" + &offset_utc_hour.to_string();
                message = message + " : (" + &msg.sender + ")";
                message = message + " > " + &msg.msg;

                writeln!(file, "{}", message)?;
            }
            Err(_) => println!("Erreur de création de fichier"),
        };

        Ok(1)
    }

    fn flush(&mut self) -> anyhow::Result<()> {
        // return pour que la fonction ne soit pas warningsée
        match io::Write::flush(&mut io::stdout()) {
            Ok(_) => {}
            Err(error) => println!("Erreur du flush : {}", error),
        };
        Ok(())
    }
}

pub fn test_callback(mess: Message) -> anyhow::Result<()> {
    print!("\r");
    println!("{} > {}", mess.sender, mess.msg);
    print!("strawberry > ");
    Ok(())
}


pub fn msg_polling<T: MsgOutput>(
    mut msg_output: T,
    client: &Client,
    login: (&str, &str),
) -> anyhow::Result<()> {
    // Entrée dans le chat
    match enter_tchat(client, login) {
        Ok(_) => {}
        Err(error) => {
            println!("Erreur de connection : {}", error);
            return Err(error);
        }
    };

    let len: usize = 10;

    let mut stay = true;
    let mut last_index: usize;

    // Récupération de l'indice du dernier message
    last_index = match get_last(client, login) {
        Ok(index) => index,
        Err(error) => return Err(error),
    };

    // Récupération des 10 derniers messages pour afficher dans le chat du client

    let last_message: usize;
    if last_index < 10 {
        last_message = last_index;
    } else {
        last_message = last_index - len;
    }
    match get_messages(client, login, last_message, len, test_callback) {
        Ok(_) => {}
        Err(error) => return Err(error),
    };

    let stdin = io::stdin();

    let client2 = client.clone();

    // thread pour la réception des nouveaux messages apparus sur le serveur
    thread::spawn(move || {
        loop {
            let pre_last_index = match get_last(&client2, USER_LOGIN) {
                Ok(index) => index,
                Err(_) => {
                    println!("Erreur lors de la récupération du dernier index");
                    0
                }
            };
            // On ne récupère les messages que s'il y a de nouveaux messages par rapport aux dernierx qu'on
            // avait récupérés
            if pre_last_index > last_index {
                match get_messages(
                    &client2,
                    USER_LOGIN,
                    last_index,
                    pre_last_index - last_index,
                    test_callback,
                ) {
                    Ok(_) => {}
                    Err(_) => {
                        println!("Erreur lors de la récupération des nouveaux messages")
                    }
                };
                last_index = pre_last_index;
            } else {
                thread::sleep(Duration::from_secs(1));
            }
        }
    });

    // partie principal pour l'écriture des messages vers le serveur
    while stay {
        let mut buffer = String::new();

        match msg_output.flush() {
            Ok(_) => {}
            Err(error) => println!("Erreur lors du flush : {}", error),
        };

        stdin.read_line(&mut buffer)?;

        if buffer.contains("quit()") {
            stay = false;
        } else {
            // let mut tmp_buff = "Ace : ".to_owned();
            // tmp_buff.push_str(&buffer);
            match send_message(client, login, buffer) {
                Ok(_) => {}
                Err(_) => {
                    println!("Erreur lors de l'envoi du message");
                }
            };
        }
    }

    match leave_tchat(client, login) {
        Ok(_) => {}
        Err(error) => {
            println!("Erreur de déconnection : {}", error);
            return Err(error);
        }
    };

    Ok(())
}

static USER_LOGIN: (&str,&str) = ("strawberry", "pnmmtSVHaC");

fn main() -> anyhow::Result<()> {

    // let opts = Opt::parse();
    // let login :(String,Option<String>) =(opts.user, opts pass);

    // let login = ("strawberry", "pnmmtSVHaC");
    let client_test = match build_reqwest_client("src/cert.pem") {
        Ok(client) => client,
        Err(error) => return Err(error),
    };

    let msg_output = Message {
        index: 0,
        date: 0.0,
        msg: String::new(),
        sender: String::new(),
    };

    msg_polling(msg_output, &client_test, USER_LOGIN)
}
//test