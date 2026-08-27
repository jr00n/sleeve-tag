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
const PATIENCE: Duration = Duration::from_secs(10);

/// Een draaiende Sleeve-instantie die bij het opruimen zichzelf beëindigt.
pub struct Server {
    process: Child,
    pub address: SocketAddr,
    log: Arc<Mutex<String>>,
    _root: tempfile::TempDir,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
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
        let port = free_port();

        let mut command = Command::new(env!("CARGO_BIN_EXE_sleeve-tag"));
        command
            .env_clear()
            .env("MUSIC_ROOT", root.path())
            .env("PORT", port.to_string())
            // De statische assets worden relatief aan de werkdirectory geserveerd.
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (name, value) in extra {
            command.env(name, value);
        }

        let mut process = command.spawn().expect("binary moet te starten zijn");
        let log = capture_output(process.stdout.take().expect("stdout is gepiped"));

        let server = Server {
            process,
            address: SocketAddr::from((Ipv4Addr::LOCALHOST, port)),
            log,
            _root: root,
        };

        server.wait_until_reachable();
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
    pub fn wait_for_log(&self, pattern: &str) -> String {
        let deadline = Instant::now() + PATIENCE;
        loop {
            let log = self.log();
            if log.contains(pattern) {
                return log;
            }
            if Instant::now() >= deadline {
                panic!("'{pattern}' verscheen niet in de log binnen {PATIENCE:?}. Log was:\n{log}");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Doet een GET-verzoek en geeft de volledige respons als tekst terug.
    pub fn get(&self, path: &str) -> String {
        let mut stream = TcpStream::connect(self.address).expect("verbinding moet lukken");
        stream
            .set_read_timeout(Some(PATIENCE))
            .expect("timeout moet in te stellen zijn");

        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            self.address
        );
        stream
            .write_all(request.as_bytes())
            .expect("verzoek moet te versturen zijn");

        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .expect("antwoord moet te lezen zijn");
        String::from_utf8_lossy(&response).into_owned()
    }

    fn wait_until_reachable(&self) {
        let deadline = Instant::now() + PATIENCE;
        while Instant::now() < deadline {
            if TcpStream::connect_timeout(&self.address, Duration::from_millis(200)).is_ok() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!(
            "server luisterde niet binnen {PATIENCE:?} op {}. Log was:\n{}",
            self.address,
            self.log()
        );
    }
}

/// Start de binary en wacht tot hij vanzelf afsluit.
///
/// Bedoeld voor de gevallen waarin de app juist *niet* hoort te starten, zoals
/// een ontbrekende of ongeldige configuratie.
pub fn start_and_expect_exit(variables: &[(&str, &str)]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sleeve-tag"));
    command.env_clear().current_dir(env!("CARGO_MANIFEST_DIR"));
    for (name, value) in variables {
        command.env(name, value);
    }
    command.output().expect("binary moet te starten zijn")
}

/// Vraagt het besturingssysteem om een vrije poort.
///
/// De listener wordt meteen losgelaten; tussen dat moment en het binden door de
/// server zit een theoretische race, maar elke test krijgt zijn eigen poort.
fn free_port() -> u16 {
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
fn capture_output(stream: std::process::ChildStdout) -> Arc<Mutex<String>> {
    let buffer = Arc::new(Mutex::new(String::new()));
    let writer = Arc::clone(&buffer);

    std::thread::spawn(move || {
        for line in BufReader::new(stream).lines() {
            let Ok(line) = line else { break };
            let mut buffer = writer.lock().expect("log-buffer moet schrijfbaar zijn");
            buffer.push_str(&line);
            buffer.push('\n');
        }
    });

    buffer
}
