// De twee toevoegingen die de UI in de browser krijgt: laten zien dat een
// schrijfactie bezig is, en een hoes kunnen neerslepen.
//
// Waarom dit nodig is: een tagwijziging in een FLAC van enkele gigabytes duurt
// minuten. `atomic::replace` kopieert eerst het hele bestand, en dat is geen
// verspilling maar de prijs van de garantie dat het origineel nooit halverwege
// kapot is. In die tijd staat de pagina stil — zonder dit bestand ziet een
// gebruiker niet of de app bezig is of vastzit, en start een tweede klik een
// tweede schrijfactie op hetzelfde bestand.
//
// Dit is een toevoeging, geen voorwaarde: zonder JavaScript versturen de
// formulieren zich precies zoals ze dat altijd deden.
//
// Twee soorten formulieren komen hier langs. Het bewerkformulier en de
// hoespagina versturen zich gewoon en laden een nieuwe pagina; daar blijft de
// bezig-staat staan tot het antwoord er is. De albumweergave gaat via htmx en
// vervangt zichzelf door het antwoord — daar verdwijnt de bezig-staat met het
// oude formulier mee, en hoeft er niets opgeruimd te worden.

(function () {
  "use strict";

  // Alleen knoppen die werkelijk schrijven dragen dit attribuut; de waarde is
  // de tekst die de knop tijdens het werk toont. Zo staat in de template, naast
  // de knop zelf, of een actie traag genoeg is om te melden — en hoeft dit
  // bestand geen enkele actienaam te kennen.
  var BEZIG_ATTRIBUUT = "data-bezig";

  /** Markering op het formulier zolang er een verzoek loopt. */
  var BEZIG_KLASSE = "is-bezig";

  var TOELICHTING =
    "Een groot bestand wordt in zijn geheel gekopieerd voordat het vervangen " +
    "wordt; bij enkele gigabytes duurt dat een paar minuten. Laat dit venster " +
    "open staan.";

  document.addEventListener("submit", function (event) {
    var form = event.target;
    if (!(form instanceof HTMLFormElement)) {
      return;
    }

    // Al bezig: dit is een tweede klik, of Enter in een tekstveld terwijl de
    // eerste schrijfactie nog loopt. Het verzoek gaat niet uit.
    if (form.classList.contains(BEZIG_KLASSE)) {
      event.preventDefault();
      return;
    }

    // `submitter` is de knop waarop geklikt is. Ontbreekt hij (Enter in een
    // veld), dan geldt de eerste knop die zou schrijven.
    var knop = event.submitter || form.querySelector("[" + BEZIG_ATTRIBUUT + "]");
    if (!knop || !knop.hasAttribute(BEZIG_ATTRIBUUT)) {
      // Een hulpactie: selecteren, hernummeren, annuleren. Die zijn meteen
      // klaar en horen geen spinner te krijgen.
      return;
    }

    form.classList.add(BEZIG_KLASSE);
    toonBezig(form, knop);
  });

  // Firefox en Safari halen een pagina uit de bfcache terug zoals hij was —
  // inclusief uitgeschakelde knoppen. Wie op "terug" drukt, hoort geen dood
  // formulier aan te treffen.
  window.addEventListener("pageshow", function (event) {
    if (event.persisted) {
      herstel();
    }
  });

  /** Zet de knop op "bezig" en meldt eronder waarom het even duurt. */
  function toonBezig(form, knop) {
    // Het oorspronkelijke opschrift bewaren, zodat `herstel` de knop kan
    // teruggeven zoals hij was.
    knop.setAttribute("data-was", knop.textContent);
    knop.innerHTML =
      '<span class="knop__spinner" aria-hidden="true"></span>' +
      escape(knop.getAttribute(BEZIG_ATTRIBUUT));

    // Pas ná deze tik uitschakelen. De browser stelt het verzoek samen tussen
    // deze functie en de volgende taak in de wachtrij; een knop die nú al
    // `disabled` is, stuurt zijn `name`/`value` niet mee — en juist daaraan
    // ziet de server welke actie bedoeld was.
    window.setTimeout(function () {
      var knoppen = form.querySelectorAll("button, input[type=submit]");
      for (var i = 0; i < knoppen.length; i++) {
        knoppen[i].disabled = true;
      }
    }, 0);

    form.appendChild(maakMelding());
  }

  /** De uitleg onder het formulier, als statusregel voor schermlezers. */
  function maakMelding() {
    var melding = document.createElement("p");
    melding.className = "bezigmelding";
    melding.setAttribute("role", "status");
    melding.textContent = TOELICHTING;
    return melding;
  }

  /** Draait alles terug wat `toonBezig` heeft gedaan. */
  function herstel() {
    var formulieren = document.querySelectorAll("form." + BEZIG_KLASSE);
    for (var i = 0; i < formulieren.length; i++) {
      var form = formulieren[i];
      form.classList.remove(BEZIG_KLASSE);

      var knoppen = form.querySelectorAll("button, input[type=submit]");
      for (var j = 0; j < knoppen.length; j++) {
        knoppen[j].disabled = false;
      }

      var melding = form.querySelector(".bezigmelding");
      if (melding) {
        melding.remove();
      }
    }

    // De opschriften zelf komen uit de bfcache zoals ze waren; alleen de knop
    // die we hebben omgezet, is niet meer wat hij was. Een herladen is hier
    // te grof — de tekst staat in `data-was`, gezet bij het omzetten.
    var omgezet = document.querySelectorAll("[data-was]");
    for (var k = 0; k < omgezet.length; k++) {
      omgezet[k].textContent = omgezet[k].getAttribute("data-was");
      omgezet[k].removeAttribute("data-was");
    }
  }

  /** Maakt tekst veilig voor gebruik in HTML. */
  function escape(tekst) {
    var doos = document.createElement("span");
    doos.textContent = tekst;
    return doos.innerHTML;
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Een hoes neerslepen
  //
  // Het slepen vult het bestandsveld dat er al staat, en verder niets: de
  // vinkjes en de knoppen bepalen nog steeds wat er met de afbeelding gebeurt.
  // Er wordt dus niets geüpload voordat er op een knop is gedrukt.
  //
  // Zonder JavaScript blijft de bestandsinvoer gewoon werken. De uitnodiging om
  // te slepen staat daarom `hidden` in de template en komt hier tevoorschijn:
  // een hint die nergens toe leidt is erger dan geen hint.
  // ─────────────────────────────────────────────────────────────────────────

  /** Wat er ingebed mag worden; dezelfde twee die `art::prepare` accepteert. */
  var TOEGESTAAN = ["image/jpeg", "image/png"];

  /** Markering op het vak zolang er iets boven zweeft. */
  var SLEEP_KLASSE = "is-sleepdoel";

  document.addEventListener("DOMContentLoaded", function () {
    var vakken = document.querySelectorAll("[data-neerzetvak]");
    for (var i = 0; i < vakken.length; i++) {
      maakNeerzetvak(vakken[i]);
    }

    // Een bestand dat naast het vak belandt, zou de browser openen — en dan is
    // de pagina met het half ingevulde formulier weg. Alleen voorkomen wanneer
    // er ook werkelijk een neerzetvak op deze pagina staat.
    if (vakken.length > 0) {
      window.addEventListener("dragover", weiger);
      window.addEventListener("drop", weiger);
    }
  });

  /** Laat de browser een gesleept bestand niet zelf openen. */
  function weiger(event) {
    event.preventDefault();
  }

  /** Maakt van één vak een sleepdoel. */
  function maakNeerzetvak(vak) {
    var invoer = vak.querySelector("input[type=file]");
    if (!invoer) {
      return;
    }

    var hint = vak.querySelector("[data-neerzetvak-hint]");
    if (hint) {
      hint.hidden = false;
    }

    vak.addEventListener("dragenter", function (event) {
      event.preventDefault();
      vak.classList.add(SLEEP_KLASSE);
    });

    vak.addEventListener("dragover", function (event) {
      // Zonder dit weigert de browser de drop: de standaardactie van dragover
      // is "hier mag niets neergezet worden".
      event.preventDefault();
      vak.classList.add(SLEEP_KLASSE);
    });

    vak.addEventListener("dragleave", function (event) {
      // `dragleave` vuurt ook bij het oversteken naar een kindelement; alleen
      // opruimen wanneer de muis het vak zelf verlaat.
      if (!vak.contains(event.relatedTarget)) {
        vak.classList.remove(SLEEP_KLASSE);
      }
    });

    vak.addEventListener("drop", function (event) {
      event.preventDefault();
      vak.classList.remove(SLEEP_KLASSE);
      neerzetten(vak, invoer, event.dataTransfer);
    });

    // Ook wie via de bestandskiezer kiest, hoort te zien wát hij gekozen heeft
    // — en te horen dat het te groot is, langs dezelfde weg.
    invoer.addEventListener("change", function () {
      if (!invoer.files || invoer.files.length !== 1) {
        return;
      }

      var bestand = invoer.files[0];
      var bezwaar = bezwaarTegen(vak, bestand);
      if (bezwaar) {
        wijsAf(vak, invoer, bezwaar);
        return;
      }

      toon(vak, bestand);
    });
  }

  /**
   * Wijst af wat er is aangeboden, en laat niets halfs achter.
   *
   * Ook het veld wordt geleegd: stond er al een geldige afbeelding klaar, dan
   * zou een knop blijven staan terwijl de melding zegt dat er iets mis is. Eén
   * duidelijke toestand is beter dan twee die elkaar tegenspreken.
   */
  function wijsAf(vak, invoer, reden) {
    invoer.value = "";
    verbergGekozen(vak);
    meld(vak, reden);
  }

  /**
   * Wat er tegen dit bestand is, of `null` wanneer het mag.
   *
   * De omvang wordt hier gecontroleerd en niet pas op de server. Een upload
   * boven de grens wordt daar afgekapt terwijl de browser nog aan het versturen
   * is; die gooit het antwoord dan weg en toont een netwerkfout, dus de uitleg
   * die de server meestuurt komt nooit aan. Beter hier tegenhouden, vóór er een
   * byte de deur uit gaat.
   */
  function bezwaarTegen(vak, bestand) {
    if (TOEGESTAAN.indexOf(bestand.type) === -1) {
      return (
        "Alleen JPEG en PNG kunnen als hoes worden ingebed; dit is " +
        (bestand.type || "een onbekend bestandstype") +
        "."
      );
    }

    var grens = parseInt(vak.getAttribute("data-max-mb"), 10);
    if (grens > 0 && bestand.size > grens * 1024 * 1024) {
      return (
        "Deze afbeelding is " +
        leesbareOmvang(bestand.size) +
        "; er gaat hoogstens " +
        grens +
        " MB in. Verklein hem eerst, of verhoog MAX_UPLOAD_MB."
      );
    }

    return null;
  }

  /** Haalt de voorbeeldweergave en alles wat erbij hoorde weer weg. */
  function verbergGekozen(vak) {
    var gekozen = vak.querySelector("[data-neerzetvak-gekozen]");
    if (gekozen) {
      gekozen.hidden = true;
    }

    var klaar = vak.querySelectorAll("[data-neerzetvak-klaar]");
    for (var i = 0; i < klaar.length; i++) {
      klaar[i].hidden = true;
    }
  }

  /** Verwerkt wat er is neergezet. */
  function neerzetten(vak, invoer, overdracht) {
    if (!overdracht || !overdracht.files || overdracht.files.length === 0) {
      // Een sleepactie zonder bestand: tekst uit een ander venster, een link.
      wijsAf(vak, invoer, "Dat is geen bestand. Sleep een JPEG of PNG hierheen.");
      return;
    }

    if (overdracht.files.length > 1) {
      // Er gaat één hoes tegelijk in een bestand. Stilzwijgend de eerste pakken
      // zou betekenen dat de gebruiker een andere afbeelding krijgt dan hij
      // dacht neer te zetten.
      wijsAf(vak, invoer, "Er gaat één hoes tegelijk. Sleep er één, niet meerdere.");
      return;
    }

    var bestand = overdracht.files[0];
    var bezwaar = bezwaarTegen(vak, bestand);
    if (bezwaar) {
      wijsAf(vak, invoer, bezwaar);
      return;
    }

    // Hier komt het bestand in het veld terecht dat het formulier tóch al
    // verstuurt. Vanaf dat moment is er geen verschil meer met een bestand dat
    // via de bestandskiezer is gekozen.
    invoer.files = overdracht.files;
    toon(vak, bestand);
  }

  /** Toont welk bestand er klaarstaat, met een voorbeeldweergave. */
  function toon(vak, bestand) {
    meld(vak, null);

    // Wat er zichtbaar wordt zodra er iets klaarstaat, bepaalt de template.
    // Op de hoespagina staat de knop er al; op de bewerkpagina verschijnt hij
    // pas hier, want daar is het hoesje in rust gewoon een plaatje.
    var klaar = vak.querySelectorAll("[data-neerzetvak-klaar]");
    for (var i = 0; i < klaar.length; i++) {
      klaar[i].hidden = false;
    }

    var gekozen = vak.querySelector("[data-neerzetvak-gekozen]");
    if (!gekozen) {
      return;
    }

    // De voorbeeldweergave bestaat pas zodra er iets te tonen valt: een lege
    // `<img>` in de pagina zou een bestand zonder hoes een afbeelding geven
    // die er nooit komt.
    var voorbeeld = gekozen.querySelector("img");
    var naam = gekozen.querySelector("figcaption");
    if (!voorbeeld) {
      voorbeeld = document.createElement("img");
      voorbeeld.className = "neerzetvak__voorbeeld";
      voorbeeld.width = 96;
      voorbeeld.height = 96;

      naam = document.createElement("figcaption");
      naam.className = "neerzetvak__naam";

      gekozen.appendChild(voorbeeld);
      gekozen.appendChild(naam);
    }

    // De vorige URL vrijgeven; anders houdt de browser elke gekozen afbeelding
    // vast tot de pagina weg is.
    if (voorbeeld.src && voorbeeld.src.indexOf("blob:") === 0) {
      URL.revokeObjectURL(voorbeeld.src);
    }

    voorbeeld.src = URL.createObjectURL(bestand);
    voorbeeld.alt = "Voorbeeld van " + bestand.name;
    naam.textContent = bestand.name + " · " + leesbareOmvang(bestand.size);
    gekozen.hidden = false;
  }

  /** Zet een melding in het vak, of haalt hem weg met `null`. */
  function meld(vak, tekst) {
    var melding = vak.querySelector("[data-neerzetvak-melding]");
    if (!melding) {
      return;
    }

    melding.textContent = tekst || "";
    melding.hidden = !tekst;
  }

  /** Een bestandsomvang zoals een mens hem leest. */
  function leesbareOmvang(bytes) {
    if (bytes < 1024) {
      return bytes + " B";
    }
    if (bytes < 1024 * 1024) {
      return Math.round(bytes / 1024) + " kB";
    }
    return (bytes / (1024 * 1024)).toFixed(1).replace(".", ",") + " MB";
  }
})();
