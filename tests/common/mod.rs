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

/// Hoe vaak het starten opnieuw geprobeerd wordt met een andere poort.
///
/// Tussen het vrijgeven van een poort door [`free_port`] en het binden ervan
/// door de server zit een gaatje waarin een andere test dezelfde poort kan
/// pakken. Met meerdere integratiebinaries naast elkaar gebeurt dat zelden,
/// maar wél. De server stopt dan meteen met een bind-fout; dit is de enige
/// plek die dat kan opvangen.
const START_ATTEMPTS: usize = 5;

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
    /// Start de binary met een lege wegwerp-bibliotheek als `MUSIC_ROOT`.
    pub fn start(extra: &[(&str, &str)]) -> Server {
        let root = tempfile::tempdir().expect("tempdir moet aan te maken zijn");
        Server::start_in(root, extra)
    }

    /// Start de binary met `root` als `MUSIC_ROOT`, een vrije poort en de
    /// opgegeven extra omgevingsvariabelen.
    ///
    /// `root` blijft eigendom van de server en wordt bij het opruimen verwijderd.
    /// Het is altijd een tempdir: een test raakt nooit de echte bibliotheek.
    ///
    /// `env_clear` zorgt dat de test niet afhangt van wat er toevallig in de
    /// shell van de ontwikkelaar of de CI-runner staat.
    pub fn start_in(root: tempfile::TempDir, extra: &[(&str, &str)]) -> Server {
        for attempt in 1..=START_ATTEMPTS {
            let port = free_port();
            let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));

            let mut process = spawn(root.path(), port, extra);
            let log = capture_output(process.stdout.take().expect("stdout is gepiped"));

            if wait_until_listening(&mut process, address, &log) {
                return Server {
                    process,
                    address,
                    log,
                    _root: root,
                };
            }

            // De server stopte voordat hij luisterde. Was de poort al bezet,
            // dan lukt het met een andere wel; ging er iets anders mis, dan
            // faalt de volgende poging op dezelfde manier en komt de log
            // hieronder alsnog boven water.
            let _ = process.kill();
            let _ = process.wait();

            if attempt == START_ATTEMPTS {
                panic!(
                    "server startte {START_ATTEMPTS} keer niet op. Laatste log was:\n{}",
                    log.lock().expect("log-buffer moet leesbaar zijn")
                );
            }
        }

        unreachable!("de lus keert terug of paniekt bij de laatste poging")
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
        self.get_with_headers(path, &[])
    }

    /// Doet een GET-verzoek en geeft de volledige respons als bytes terug.
    ///
    /// Nodig zodra het antwoord geen tekst is: een JPEG overleeft het niet om
    /// als UTF-8 gelezen te worden.
    pub fn get_bytes(&self, path: &str) -> Vec<u8> {
        self.request(path, &[])
    }

    /// Zoals [`Server::get`], met extra verzoekheaders.
    ///
    /// Nodig voor `HX-Request`: die header bepaalt of de server de hele pagina
    /// of alleen het te vervangen fragment teruggeeft.
    pub fn get_with_headers(&self, path: &str, headers: &[(&str, &str)]) -> String {
        String::from_utf8_lossy(&self.request(path, headers)).into_owned()
    }

    fn request(&self, path: &str, headers: &[(&str, &str)]) -> Vec<u8> {
        let mut stream = TcpStream::connect(self.address).expect("verbinding moet lukken");
        stream
            .set_read_timeout(Some(PATIENCE))
            .expect("timeout moet in te stellen zijn");

        let extra: String = headers
            .iter()
            .map(|(name, value)| format!("{name}: {value}\r\n"))
            .collect();

        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: {}\r\n{extra}Connection: close\r\n\r\n",
            self.address
        );
        stream
            .write_all(request.as_bytes())
            .expect("verzoek moet te versturen zijn");

        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .expect("antwoord moet te lezen zijn");
        response
    }
}

/// Start de binary met de opgegeven bibliotheek en poort.
fn spawn(root: &std::path::Path, port: u16, extra: &[(&str, &str)]) -> Child {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sleeve-tag"));
    command
        .env_clear()
        .env("MUSIC_ROOT", root)
        .env("PORT", port.to_string())
        // De statische assets worden relatief aan de werkdirectory geserveerd.
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for (name, value) in extra {
        command.env(name, value);
    }

    command.spawn().expect("binary moet te starten zijn")
}

/// Wacht tot *ons eigen* serverproces meldt dat het luistert.
///
/// `false` betekent dat het proces eerst is gestopt; de aanroeper probeert het
/// dan met een andere poort.
///
/// Waarom de log en niet gewoon een verbinding: een geslaagde verbinding zegt
/// alleen dat er *iets* op die poort luistert. Draaien er meerdere
/// integratiebinaries naast elkaar, dan kan dat de server van een andere test
/// zijn die dezelfde poort te pakken had — en dan praat deze test tegen een
/// vreemde bibliotheek, of valt hij om zodra die andere server afsluit. Precies
/// dat gebeurde, in ongeveer één op de drie volledige testruns.
///
/// Op één poort kan maar één proces luisteren. Zegt onze eigen stdout dat hij
/// gebonden is, dan is de socket dus van ons. Lukte het binden niet, dan stopt
/// het proces en vangt `try_wait` dat op.
fn wait_until_listening(
    process: &mut Child,
    address: SocketAddr,
    log: &Arc<Mutex<String>>,
) -> bool {
    let deadline = Instant::now() + PATIENCE;

    // De server bindt op alle interfaces; de test praat via de loopback.
    let bound = format!("0.0.0.0:{}", address.port());

    while Instant::now() < deadline {
        match process.try_wait() {
            Ok(Some(_)) => return false,
            Ok(None) => {}
            Err(error) => panic!("kon de status van het serverproces niet lezen: {error}"),
        }

        {
            let log = log.lock().expect("log-buffer moet leesbaar zijn");
            if log.contains("webserver luistert") && log.contains(&bound) {
                return true;
            }
        }

        std::thread::sleep(Duration::from_millis(20));
    }

    panic!(
        "server luisterde niet binnen {PATIENCE:?} op {address}. Log was:\n{}",
        log.lock().expect("log-buffer moet leesbaar zijn")
    );
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

/// Pad naar een fixture in de repo.
///
/// De integratietests kunnen `src/testfixtures.rs` niet gebruiken: dat hoort bij
/// de binary-crate. De namen staan daar wel beschreven.
pub fn fixture_path(name: &str) -> std::path::PathBuf {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);

    assert!(
        path.is_file(),
        "fixture '{name}' ontbreekt op {}. Genereer hem opnieuw met tests/fixtures/genereer-fixtures.sh",
        path.display()
    );

    path
}

/// Kopieert een fixture naar `directory` onder de opgegeven naam.
pub fn place_fixture(directory: &std::path::Path, name: &str, fixture: &str) {
    std::fs::copy(fixture_path(fixture), directory.join(name)).unwrap_or_else(|error| {
        panic!(
            "fixture '{fixture}' kon niet als '{name}' in {} gezet worden: {error}",
            directory.display()
        )
    });
}
