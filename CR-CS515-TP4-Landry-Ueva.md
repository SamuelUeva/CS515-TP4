# CS515 - TP4
LANDRY Jules
UEVA Samuel
_____________

## Question 1 
Voici notre implémentation du trait MsgOutput sur les trois structures suivantes : 
- JsonOutput - Devant écrire les logs au format JSON dans un fichier
- CSVOutput - Devant écrire les logs au format CSV dans un fichier
- SyslogOutput - Devant envoyer les logs dans le syslog du système.

```Rust
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
                    writeln!(file, "Date,Sender,Index,Message")?;
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

        let formatter = Formatter3164 {
            facility: Facility::LOG_USER,
            hostname: None,
            process: "CS515-tp4_messagerie".into(),
            pid: process::id(),
        };

        let offset_utc_hour = 2.0;
        let offset_utc_sec: f64 = offset_utc_hour * 3600.0;
        let mut message = String::new();
        let date_updated = Utc.timestamp((msg.date + offset_utc_sec) as i64, 0);
        message = message + &date_updated.to_string() + "+" + &offset_utc_hour.to_string();
        message = message + " " + &msg.sender + "["+ &msg.index.to_string() +"]" ;
        message = message + " " + &msg.msg;
        
        match syslog::unix(formatter) {
            Err(e) => println!("impossible to connect to syslog: {:?}", e),
            Ok(mut writer) => {
                match writer.err(message){
                Ok(_) => {},
                Err(error) => println!("Error for writting for syslog: {}", error)
                };
            }
        }
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
```
Nous avons ajouté une fonction de clonnage pour MsgOutput pour pouvoir clonner les objets de type Box<dyn\>.

## Question 2

On crée une nouvelle structure pour accueillir les éléments : 
```rust
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
```
Ensuite dans le main on va parser la commande tapée afin de recueillir les paramètres : 

```Rust
    let args = Args::parse();
    let login = (args.user.as_str(),args.pass.as_str());
```

Avec ces lignes de codes, le "user" et le "pass" sont obligatoire car par défaut c'est obligatoire. Pour rendre des paramètres non obligatoire il faut mettre le type dans un "Option" (exemple: Option<String\>). Par contre les options d'output étant des booléens, elles sont par défaut optionnelles.

En raison de problèmes nous avons du utiliser la crate lazy_static pour initialiser des variables static pour que le nom de l'utilisateur et le mot de passe passés en arguments puissent être utilisés par le thread récupérant les messages.
Nous n'avons pas réussi à clonner et utiliser les mutex pour utiliser les logins dans ce thread.

## Question 3
Nous avons créé dans le 'main' un vecteur contenant les structures des formats que l'utilisateur a passés en argument de la commande. Ce vecteur est ensuite passé dans les fonctions pour qu'on puisse l'utiliser lors de l'écriture des messages.
Cependant, nous n'avons pas réussi à faire passer correctement ce vecteur dans le thread que nous utilisons. Nous avons du faire 2 copies de ce vecteur pour qu'il ne soit pas 'moved' après un tour de boucle. Une copie qui est passée en argument et une copie pour en refaire des copies.
Nous avons également mis en place ce procédé lors de l'utilisation de ce vecteur contenant les formats souhaités lors de l'écriture de messages.

Nous avons au départ passé en paramètre de fonction les valeurs booléennes des formats pour créer le vecteur en question, seulement avant la phase d'écriture des messages.
Nous avons toutefois estimé que recréer ce vecteur plusieurs fois, à chaque appel de la fonction `get_message`, n'était sûrement pas la bonne approche.
Nous nous sommes retrouvé ensuite dans la difficulté de cloner ce vecteur pour pouvoir le passer en argument de fonction. Nous avons donc créé une fonction de clonage (évoqué précédemment) pour cloner les éléments du vecteur pour réaliser le procédé mentionné plus tôt.