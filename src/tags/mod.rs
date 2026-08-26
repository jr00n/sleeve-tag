//! Tag-I/O en het genormaliseerde tagmodel.
//!
//! Dit is de enige module die `lofty` aanroept en de enige module die
//! audiobestanden muteert. De rest van de applicatie werkt uitsluitend met het
//! genormaliseerde model en weet niet of een bestand ID3v2-frames of
//! Vorbis-comments bevat.
//!
//! Vaste regels uit het PRD:
//! - MP3 wordt altijd weggeschreven als ID3v2.4 (UTF-8); ID3v1 wordt verwijderd
//!   of gesynchroniseerd, nooit inconsistent achtergelaten.
//! - Niet-gemodelleerde tags blijven ongewijzigd bewaard.
//! - Een leeg veld betekent "veld verwijderen", niet "lege waarde opslaan".
//!
//! Deze module wordt ingevuld door de lees- en schrijftaken van fase 1 en 2.
