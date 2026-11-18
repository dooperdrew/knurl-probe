---
title: Risultati sulla WH-1000XM3
title_en: WH-1000XM3 results
summary: Cosa abbiamo verificato sulla XM3 dell'utente e cosa resta aperto.
summary_en: What we verified on the user's XM3 and what is still open.
type: record
topic: verification
status: current
verified: 2026-11-08
order: 60
---

# Risultati sull'hardware: WH-1000XM3

Cuffie dell'utente, firmware **4.5.2**, PC Windows 10 Pro 19045. Test del 2026-11-07 con `xmc-probe`. I frame citati sono nei test `crates/xmc-protocol/tests/xm3_capture_2026_11_07.rs`. Il file di cattura completo non è nel repository perché contiene l'indirizzo Bluetooth.

## Verificato
| Domanda | Risultato |
|---|---|
| Quale servizio MDR? | **v1** (`96cc203e-…`). `rfcomm-check`: v1 si apre, v2 dà WSA 10049 (assente) |
| Come lo si raggiunge su Windows? | **Solo con Winsock** (`AF_BTH`). Le API WinRT non lo elencano, nemmeno con la ricerca SDP senza cache, che mostra solo altri 5 servizi (Hands-Free, Headset, GAIA di Qualcomm e due servizi generici) |
| Framing, checksum, escape | Ogni frame ricevuto si decodifica e si ricodifica identico, compresi i frame con sequenza 1 |
| ACK dell'host (`1 - seq`, payload vuoto) | Accettato: dopo il nostro ACK la cuffia non ripete le risposte (tranne un doppione dell'EQ a 10 ms, vedi sotto) |
| Sequenza del comando successivo | Usare la sequenza dell'ACK della cuffia funziona per 7 richieste di fila |
| La risposta può arrivare prima dell'ACK | Sì, per `00 00` e per `04 01`/`04 02` |

## Risposte alle richieste di lettura
| Richiesta | Risposta (payload) | Lettura |
|---|---|---|
| `00 00` versione protocollo | `01 00 40 10` | Identica alla cattura XM3 pubblicata |
| `04 01` modello | `05 01 0A` + `WH-1000XM3` | |
| `04 02` firmware | `05 02 05` + `4.5.2` | |
| `06 00` funzioni supportate | `07 00 13` + 19 codici | Codici ancora da interpretare |
| `10 00` batteria | `11 00 64 00` | 100 %, non in carica (da confermare con un altro livello) |
| `66 02` NC/Ambient | `67 02 01 02 02 01 00 00` | Uguale al frame pubblicato |
| `56 01` equalizzatore | `57 01 00 06 0A 0A 0A 0A 0A 0A` | Preset `00` (spento?), 6 bande tutte a `0A` (centro?) |

## Notifiche spontanee
- In 70 secondi tra il passo 1 e il passo 2 (cuffie ferme, poi 3 pressioni del tasto NC) non è arrivato **nessun** frame.
- A 76 s, durante il passo 3, è arrivata la notifica `69 02 01 02 00 01 00 14` **11 volte in circa 15 ms**, ognuna con la nostra ACK. Subito dopo Windows ha chiuso la connessione (WSA 10053) e i passi 4-7 non hanno registrato nulla.
- Rispetto alla risposta `67 02`, la notifica cambia il terzo byte da `02` a `00` e l'ultimo da `00` a `14` (20): probabilmente il passaggio ad Ambient Sound con livello 20. Da confermare.

## Prova con l'app (2026-11-07, sera)
Prova dell'utente con `xmcompanion.exe` (build di rilascio, sola lettura). L'app non registra i byte: quanto segue è ciò che l'utente ha visto nella finestra.

- **Tasto NC:** passando tra le modalità con il tasto delle cuffie, la riga "Noise" ha mostrato ogni volta la modalità giusta. Le notifiche del tasto quindi arrivano e si decodificano, questa volta senza il ritardo della prima cattura.
- **Batteria:** 90 %, non in carica.
- **Modello:** la richiesta `04 01` ha ricevuto l'ACK ma **nessuna risposta** entro 5 s ("Request 0x04 failed: no reply arrived"). Nella cattura del mattino la stessa richiesta aveva risposto. Causa non nota (collegamento instabile?). Correzione: una richiesta con ACK e senza risposta ora viene ripetuta una volta prima di dichiararla fallita (`NO_REPLY_RETRIES`). Da verificare se basta.
- **Più copie dell'app:** si erano aperte più copie insieme. Solo una può tenere il canale MDR; le altre mostravano "connect failed: WSA error 10048". Correzione: l'app ora si avvia una sola volta per sessione di Windows e la seconda copia spiega dove trovare la prima.

### Seconda prova con le correzioni (2026-11-07, sera)
Build di rilascio con il commit `f3edf1e`. Esito riferito dall'utente:

| Controllo | Esito |
|---|---|
| All'avvio arrivano tutti i valori, modello compreso | Ok |
| Una seconda copia mostra l'avviso e non apre un'altra finestra | Ok |
| "About XMCompanion" dal menu del tray apre la finestra con la versione e il riquadro di Slint | Ok |
| Cuffie spente e riaccese con l'app aperta: "Not connected", poi di nuovo "Connected" con i valori | Ok |

Non sappiamo se il modello questa volta sia arrivato al primo tentativo o grazie alla ripetizione: l'app non registra i byte.

Prova successiva (commit `0e3a1f5`): "Open" e "About" dal tray portano davanti la finestra anche se è già aperta e coperta (verificato dall'utente). Nella stessa prova la finestra ha mostrato in rosso "opcode 0xA9, expected 4 bytes, got 3": la XM3 ha mandato una notifica `A9` di 3 byte, mentre il decoder conosceva solo il formato del volume (`A9 01 20 vv`, dalle fonti, mai visto da noi). I byte non sono stati registrati. Correzione: gli altri formati `A9` restano "sconosciuti" invece di dare errore, e ogni errore di decodifica ora riporta i byte ricevuti. Da capire con una cattura cosa significa quella notifica (probabilmente lo stato di riproduzione: il browser aveva appena aperto una scheda).

## Ancora aperto
1. **Perché nella prima cattura le pressioni del tasto NC non hanno generato notifiche subito, e perché la notifica è arrivata ripetuta 11 volte.** Nella prova con l'app le notifiche sono arrivate regolarmente, il che rafforza l'ipotesi di un collegamento fermo in quel momento. Non verificato con un log dei byte.
2. **Il passo 3 (togliere le cuffie) non è significativo sulla XM3**, che per quanto sappiamo non ha il sensore di indossamento (non verificato).
3. **Il passo 7 (cavo di ricarica) non è registrabile sulla XM3**: collegata al cavo USB, la cuffia spegne il Bluetooth (osservato dall'utente il 2026-11-07).
4. Musica, Quick Attention: da rifare, la connessione era già chiusa.
