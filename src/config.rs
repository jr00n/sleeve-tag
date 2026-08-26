//! Configuratie van de applicatie, uitsluitend gelezen uit omgevingsvariabelen.
//!
//! Sleeve draait als container zonder configuratiebestand: `MUSIC_ROOT`, `PORT`,
//! `PUID`/`PGID`, `MAX_ART_SIZE`, `LOG_LEVEL` en `BACKUP_ON_WRITE` bepalen samen
//! het volledige gedrag. In de container is `MUSIC_ROOT` altijd `/music`; het
//! pad van de muziekshare op de host is puur een volume-mount en is de app
//! onbekend.
//!
//! Deze module wordt ingevuld door de configuratietaak van fase 0.
