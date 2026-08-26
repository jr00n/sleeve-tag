//! Gedeelde hulpmiddelen voor de integratietests.
//!
//! De binary is sinds de webserver-taak een langlopend proces: hij sluit niet
//! meer uit zichzelf af. Tests kunnen dus niet op `Output` wachten, maar moeten
//! meelezen met de uitvoer en het proces zelf beëindigen. Die afhandeling staat
//! hier, zodat elke integratietest hem deelt.
//!
//! Elk testbestand compileert deze module apart en gebruikt er maar een deel
//! van; vandaar dat ongebruikte code hier is toegestaan.

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Hoe lang een test maximaal op de server of op een logregel wacht.
const GEDULD: Duration = Duration::from_secs(10);

/// Een draaiende Sleeve-instantie die bij het opruimen zichzelf beëindigt.
pub struct Server {
    proces: Child,
    pub adres: SocketAddr,
    log: Arc<Mutex<String>>,
    _root: tempfile::TempDir,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.proces.kill();
        let _ = self.proces.wait();
    }
}

impl Server {
    /// Start de binary met een lege omgeving plus `MUSIC_ROOT`, een vrije poort
    /// en de opgegeven extra variabelen.
    ///
    /// `env_clear` zorgt dat de test niet afhangt van wat er toevallig in de
    /// shell van de ontwikkelaar of de CI-runner staat.
    pub fn start(extra: &[(&str, &str)]) -> Server {
        let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
        let poort = vrije_poort();

        let mut commando = Command::new(env!("CARGO_BIN_EXE_sleeve-tag"));
        commando
            .env_clear()
            .env("MUSIC_ROOT", root.path())
            .env("PORT", poort.to_string())
            // De statische assets worden relatief aan de werkdirectory geserveerd.
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (naam, waarde) in extra {
            commando.env(naam, waarde);
        }

        let mut proces = commando.spawn().expect("binary moet te starten zijn");
        let log = lees_mee(proces.stdout.take().expect("stdout is gepiped"));

        let server = Server {
            proces,
            adres: SocketAddr::from((Ipv4Addr::LOCALHOST, poort)),
            log,
            _root: root,
        };

        server.wacht_tot_bereikbaar();
        server
    }

    /// Alles wat de server tot nu toe naar stdout heeft geschreven.
    pub fn log(&self) -> String {
        self.log
            .lock()
            .expect("log-buffer moet leesbaar zijn")
            .clone()
    }

    /// Wacht tot de log een regel met `patroon` bevat en geeft de hele log terug.
    pub fn wacht_op_log(&self, patroon: &str) -> String {
        let deadline = Instant::now() + GEDULD;
        loop {
            let log = self.log();
            if log.contains(patroon) {
                return log;
            }
            if Instant::now() >= deadline {
                panic!("'{patroon}' verscheen niet in de log binnen {GEDULD:?}. Log was:\n{log}");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Doet een GET-verzoek en geeft de volledige respons als tekst terug.
    pub fn get(&self, pad: &str) -> String {
        let mut stroom = TcpStream::connect(self.adres).expect("verbinding moet lukken");
        stroom
            .set_read_timeout(Some(GEDULD))
            .expect("timeout moet in te stellen zijn");

        let verzoek = format!(
            "GET {pad} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            self.adres
        );
        stroom
            .write_all(verzoek.as_bytes())
            .expect("verzoek moet te versturen zijn");

        let mut antwoord = Vec::new();
        stroom
            .read_to_end(&mut antwoord)
            .expect("antwoord moet te lezen zijn");
        String::from_utf8_lossy(&antwoord).into_owned()
    }

    fn wacht_tot_bereikbaar(&self) {
        let deadline = Instant::now() + GEDULD;
        while Instant::now() < deadline {
            if TcpStream::connect_timeout(&self.adres, Duration::from_millis(200)).is_ok() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!(
            "server luisterde niet binnen {GEDULD:?} op {}. Log was:\n{}",
            self.adres,
            self.log()
        );
    }
}

/// Start de binary en wacht tot hij vanzelf afsluit.
///
/// Bedoeld voor de gevallen waarin de app juist *niet* hoort te starten, zoals
/// een ontbrekende of ongeldige configuratie.
pub fn start_en_verwacht_afsluiten(variabelen: &[(&str, &str)]) -> std::process::Output {
    let mut commando = Command::new(env!("CARGO_BIN_EXE_sleeve-tag"));
    commando.env_clear().current_dir(env!("CARGO_MANIFEST_DIR"));
    for (naam, waarde) in variabelen {
        commando.env(naam, waarde);
    }
    commando.output().expect("binary moet te starten zijn")
}

/// Vraagt het besturingssysteem om een vrije poort.
///
/// De listener wordt meteen losgelaten; tussen dat moment en het binden door de
/// server zit een theoretische race, maar elke test krijgt zijn eigen poort.
fn vrije_poort() -> u16 {
    let listener =
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("poort moet vrij te vinden zijn");
    listener
        .local_addr()
        .expect("adres moet leesbaar zijn")
        .port()
}

/// Leest de uitvoer van het proces in een achtergrondthread mee.
///
/// Nodig omdat de server blijft draaien: wachten op EOF zou de test laten
/// hangen, en een volgelopen pipe zou de server blokkeren.
fn lees_mee(stroom: std::process::ChildStdout) -> Arc<Mutex<String>> {
    let buffer = Arc::new(Mutex::new(String::new()));
    let schrijver = Arc::clone(&buffer);

    std::thread::spawn(move || {
        for regel in BufReader::new(stroom).lines() {
            let Ok(regel) = regel else { break };
            let mut buffer = schrijver.lock().expect("log-buffer moet schrijfbaar zijn");
            buffer.push_str(&regel);
            buffer.push('\n');
        }
    });

    buffer
}
