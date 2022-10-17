# TP4 - Approfissement + Decouverte

Les trois seances de travaux pratiques precedentes nous aurons permis de voir des notions importantes lors de l'ecriture de programmes en Rust.

Nous avons ainsi pu manipuler :

- Le polymorphisme statique
- Les threads
- Les channels
- L'asynchrone

Ce dernier TP aura pour but de vous faire manipuler deux nouvelles notions : 

- Le polymorphisme dynamique 
- Le parsing des arguments en ligne de commande au travers de la crate `clap`

> Reprenez votre travail realise durant le TP2 (Client de messagerie).

Nous souhaitons maintenant ne plus travailler avec une seule structure implementant le trait `MsgOutput`, mais avec un nombre arbitraire non connu a la compilation.

### Premiere question

Implementez le trait `MsgOutput` sur trois nouvelles structures :

- JsonOutput - Devant ecrire les logs au format JSON dans un fichier
- CSVOutput - Devant ecrire les logs au format CSV dans un fichier
- SyslogOutput - Devant envoyer les logs dans le syslog du systeme.

### Deuxieme question

Prenez en main la crate `clap`. Cette crate permet de generer facilement des CLI, sans avoir a utiliser la fastidieuse fonction `getopt()` de la bibliotheque standard C.

Utiliser `clap` pour demander via la CLI de facon obligatoire deux arguments, `--user` et `--pass` permettant d'envoyer le nom d'utilisateur et le mot de passe sans avoir a l'hardcoder directement dans le binaire.

### Troisieme question

Ajouter maintenant trois arguments booleens a la CLI, `--json`, `--csv`, `--syslog`.

Faites maintenant en sorte de pouvoir ecrire independemment dans chaque sortie en fonction du choix de l'utilisateur.

Exemple:

```rust
./tp3 --user kiwi --pass PasSw0Rd! --json --syslog // Ecrit dans le fichier JSON et dans SYSLOG
./tp3 --user kiwi --pass PasSw0Rd! // N'ecrit nulle part
```

### Bonus

Utilisez la crate `plotters` pour afficher en temps reel un graphe des personnes parlant le plus sur le serveur !

(Pour des raisons de simplifations, on entendra par temps reel le fait de reecrire dans un seul et meme fichier)