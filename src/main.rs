use chrono::{TimeZone, Utc};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::{thread, vec};
use std::time::Duration;
use std::{io, path::Path};
use clap::Parser;


#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
   /// Name of the user
   #[arg(short, long)]
   user: String,

   /// Password of the user
   #[arg(short, long)]
   pass: String,

   /// Set JSON format for output message
   #[arg(short,long)]
   json: bool,
   
   /// Set CSV format for output message
   #[arg(short,long)]
   csv: bool,
   
   /// Set SYSLOG format for output message
   #[arg(short,long)]
   syslog: bool,

}

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
    date: String,
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
    vec_output: Vec<Box<dyn MsgOutput>>,
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

    // Copie du vecteur des formats dans une autre variable qu'on réutilisera plus tard
    let mut tmp3 = Vec::new();
    for elt in vec_output{
        tmp3.push(elt.clone_dyn());
    }

    for elt in test_json {
        // Copie du vecteur dans deux autres variables
        // 1 pour garder une sauvegarde pendant qu'on boucle
        // Une pour l'utiliser et écrire les messages aux formats demandés
        let mut tmp1 = Vec::new();
        let mut tmp2 = Vec::new();
        for elt in &tmp3 {
            tmp1.push(elt.clone_dyn());
            tmp2.push(elt.clone_dyn());
        }

        // Récupération de la sauvegarde
        let tmp3 = tmp2;

        // On parcourt le vecteur de format pour écrire les messages
        for mut format in tmp3 {
            match (format.as_mut()).write_msg(&elt) {
                Ok(_) => {}
                Err(error) => return Err(error),
            };
        }
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

// Nous avons ajouté une fonction pour cloner les éléments de type Box<dyn> pour
// nos besoins et le trait Send
pub trait MsgOutput: Send {
    fn write_msg(&mut self, msg: &Message) -> anyhow::Result<usize>;
    fn flush(&mut self) -> anyhow::Result<()>;
    fn clone_dyn(&self) -> Box<dyn MsgOutput>;
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
        
        match io::Write::flush(&mut io::stdout()) {
            Ok(_) => {}
            Err(error) => println!("Erreur du flush : {}", error),
        };
        Ok(())
    }

    fn clone_dyn(&self) -> Box<dyn MsgOutput> {
        let msg_output_default = Message {
            index: self.index,
            date: 0.0,
            msg: String::new(),
            sender: String::new(),
        };
        Box::new(msg_output_default)
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
                let date_updated = Utc.timestamp((msg.date + offset_utc_sec) as i64, 0);
                let new_date = date_updated.to_string();
                
                let json_output_message = JsonOutput {
                    date: new_date,
                    index: msg.index,
                    msg: msg.msg.clone(),
                    sender: msg.sender.clone()
                };

                let message = serde_json::to_string(&json_output_message)?;

                writeln!(file, "{}", message)?;
            }
            Err(_) => println!("Erreur de création de fichier"),
        };
        Ok(1)
    }

    fn flush(&mut self) -> anyhow::Result<()> {
        
        match io::Write::flush(&mut io::stdout()) {
            Ok(_) => {}
            Err(error) => println!("Erreur du flush : {}", error),
        };
        Ok(())
    }
    fn clone_dyn(&self) -> Box<dyn MsgOutput> {
        let json_output = JsonOutput {
            date: self.date.clone(),
            index: self.index,
            msg: self.msg.clone(),
            sender: self.sender.clone(),
        };
        Box::new(json_output)
    }
}

impl MsgOutput for CSVOutput {
    fn write_msg(&mut self, msg: &Message) -> anyhow::Result<usize> {
        let path = "csv_output.txt";
        let already = Path::new(path).exists();
        if !already {
            match File::create(path) {
                Ok(mut file) => {
                    writeln!(file, "{}", "Date,Sender,Index,Message")?;
                }
                Err(error) => println!("Erreur de création de fichier : {}", error),
            }
        }
        match OpenOptions::new().write(true).append(true).open(path) {
            Ok(mut file) => {
                let offset_utc_hour = 2.0;
                let offset_utc_sec: f64 = offset_utc_hour * 3600.0;
                let mut message = String::new();
                let date_updated = Utc.timestamp((msg.date + offset_utc_sec) as i64, 0);
                message = message + &date_updated.to_string() + "+" + &offset_utc_hour.to_string() + ",";
                message = message + &msg.sender + ",";
                message = message + "[" + &msg.index.to_string() + "],";
                message = message + &msg.msg;
                writeln!(file, "{}", message)?;
            }
            Err(_) => println!("Erreur de création de fichier"),
        };
        Ok(1)
    }

    fn flush(&mut self) -> anyhow::Result<()> {
        
        match io::Write::flush(&mut io::stdout()) {
            Ok(_) => {}
            Err(error) => println!("Erreur du flush : {}", error),
        };
        Ok(())
    }

    fn clone_dyn(&self) -> Box<dyn MsgOutput> {
        let csv_output = CSVOutput {
            date: self.date,
            index: self.index,
            msg: self.msg.clone(),
            sender: self.sender.clone(),
        };

        Box::new(csv_output)
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
                message = message + " " + &msg.sender + "["+ &msg.index.to_string() +"]" ;
                message = message + " " + &msg.msg;
                
                writeln!(file, "{}", message)?;
            }
            Err(_) => println!("Erreur de création de fichier"),
        };

        Ok(1)
    }

    fn flush(&mut self) -> anyhow::Result<()> {
        
        match io::Write::flush(&mut io::stdout()) {
            Ok(_) => {}
            Err(error) => println!("Erreur du flush : {}", error),
        };
        Ok(())
    }

    fn clone_dyn(&self) -> Box<dyn MsgOutput> {
        let syslog_output = SyslogOutput {
            date: self.date,
            index: self.index,
            msg: self.msg.clone(),
            sender: self.sender.clone(),
        };
        Box::new(syslog_output)
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
    vec_output: Vec<Box<dyn MsgOutput + Send >>,
) -> anyhow::Result<()> {
    
    // Entrée dans le chat
    match enter_tchat(client, login) {
        Ok(_) => {}
        Err(error) => {
            println!("Erreur de connection : {}", error);
            return Err(error);
        }
    };

    let mut stay = true;
    let mut last_index: usize;

    // Récupération de l'indice du dernier message
    last_index = match get_last(client, login) {
        Ok(index) => index,
        Err(error) => return Err(error),
    };

    // Copie du vecteur dans une autre variable pour la réutiliser après
    let mut tmp3 = Vec::new();
    for elt in vec_output{
        tmp3.push(elt.clone_dyn());
    }

    let stdin = io::stdin();

    let client2 = client.clone();

    // thread pour la réception des nouveaux messages apparus sur le serveur
    thread::spawn(move || {
        
        loop {
            // Copie du vecteur dans deux autres variable. Une pour avoir une sauvegarde et la repasser dans la
            // boucle for
            // Une pour la passer à la fonction get_message
            let mut tmp = Vec::new();
            let mut tmp2 = Vec::new();
            for elt in tmp3 {
                tmp.push(elt.clone_dyn());
                tmp2.push(elt.clone_dyn());
            }

            // Récupération d'une des copies pour pouvoir continuer dans la boucle
            tmp3 = tmp2;
            
            let pre_last_index = match get_last(&client2, USER_LOGIN) {
                Ok(index) => index,
                Err(_) => {
                    println!("Erreur lors de la récupération du dernier index");
                    0
                }
            };
            // On ne récupère les messages que s'il y a de nouveaux messages par rapport aux derniers qu'on
            // avait récupérés
            if pre_last_index > last_index {
                match get_messages(
                    &client2,
                    USER_LOGIN,
                    last_index,
                    pre_last_index - last_index,
                    test_callback,
                    tmp
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

    let args = Args::parse();
    // Récupération du login et du mot de passe
    let login = (args.user.as_str(),args.pass.as_str());

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

    let mut vec_output = Vec::new();
    
    // Vérification du format à utiliser pour l'écriture des messages
    if args.json {
        let json_output = JsonOutput {
            date: "".to_string(),
            index: 0,
            msg: "".to_string(),
            sender: "".to_string(),
        };
        vec_output.push(Box::new(json_output) as Box<dyn MsgOutput + Send>);
    }
    if args.csv {
        let csv_output = CSVOutput {
            date: 0.0,
            index: 0,
            msg: "".to_string(),
            sender: "".to_string(),
        };
        vec_output.push(Box::new(csv_output)as Box<dyn MsgOutput + Send>);
    }
    if args.syslog {
        let syslog_output = SyslogOutput {
            date: 0.0,
            index: 0,
            msg: "".to_string(),
            sender: "".to_string(),
        };
        vec_output.push(Box::new(syslog_output)as Box<dyn MsgOutput + Send>);
    }

    // println!("Test argument commande : {}, {}", login.0, login.1);
    // println!("pour arguments output : {}", 1);

    msg_polling(msg_output, &client_test, login, vec_output)
}

// cargo run --bin server --release -- --adress 0.0.0.0 --port 40443 --database chat.db --output chat.txt --cert cert.perm --key key.pem