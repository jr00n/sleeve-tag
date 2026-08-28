// Laat zien dat een schrijfactie bezig is, en houdt een tweede klik tegen.
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
})();
