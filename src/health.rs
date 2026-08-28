//! De healthcheck van de container, als tweede bedrijfsmodus van dezelfde
//! binary.
//!
//! De runtime is distroless: geen shell, geen `curl`, geen `wget`. Een
//! `healthcheck:` in compose kan daar dus alleen iets aanroepen wat al in het
//! image zit, en dat is precies één bestand — deze binary. `sleeve-tag --health`
//! doet daarom zelf één verzoek aan `/healthz` op de loopback en eindigt met
//! exitcode 0 (gezond) of 1 (niet gezond), wat Docker als enige signaal nodig
//! heeft.
//!
//! Het verzoek wordt met de hand opgebouwd over een `TcpStream`. Een
//! HTTP-clientcrate erbij halen voor één `GET` naar `127.0.0.1` zou de binary
//! laten groeien voor werk dat in een handvol regels past.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

/// Hoe lang de probe hoogstens op verbinding en antwoord wacht.
///
/// Kort, en met opzet: een healthcheck die blijft hangen tot Docker hem afkapt,
/// meldt niets — hij vertraagt alleen het moment waarop een vastgelopen
/// container wordt herstart.
const TIMEOUT: Duration = Duration::from_secs(3);

/// Vraagt `/healthz` op en zegt of de app gezond antwoordde.
///
/// Altijd op de loopback: de probe draait in dezelfde netwerknamespace als de
/// server, en een healthcheck hoort niet afhankelijk te zijn van hoe de poort
/// naar buiten is doorgezet.
pub fn probe(port: u16) -> bool {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));

    match ask(address) {
        Ok(status_line) => {
            let healthy = status_line.starts_with("HTTP/1.1 200");
            if !healthy {
                eprintln!("sleeve-tag: /healthz antwoordde met '{status_line}'");
            }
            healthy
        }
        Err(error) => {
            eprintln!("sleeve-tag: /healthz op {address} is niet bereikbaar ({error})");
            false
        }
    }
}

/// Doet het verzoek en geeft de statusregel van het antwoord terug.
fn ask(address: SocketAddr) -> std::io::Result<String> {
    let mut stream = TcpStream::connect_timeout(&address, TIMEOUT)?;
    stream.set_read_timeout(Some(TIMEOUT))?;
    stream.set_write_timeout(Some(TIMEOUT))?;

    write!(
        stream,
        "GET /healthz HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
    )?;

    // Alleen de statusregel is interessant, maar het antwoord is zo klein dat
    // apart afkappen niets oplevert.
    let mut response = String::new();
    stream.read_to_string(&mut response)?;

    Ok(response.lines().next().unwrap_or_default().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_closed_port_is_not_healthy() {
        // Poort 1 is op geen enkele ontwikkelmachine in gebruik door een
        // proces dat een gebruiker zonder rechten mag starten.
        assert!(!probe(1));
    }

    // Het geslaagde geval heeft een draaiende server nodig en staat daarom in
    // tests/health.rs, dat de binary echt start.
}
