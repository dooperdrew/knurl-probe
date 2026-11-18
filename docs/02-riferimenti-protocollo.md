---
title: Riferimenti del protocollo MDR
title_en: MDR protocol references
summary: Cosa dicono le fonti pubbliche sul protocollo Sony MDR, con licenze, fonti e punti verificati sulle nostre cuffie.
summary_en: What public sources say about the Sony MDR protocol, with licenses, sources and what we verified on our headphones.
type: reference
topic: protocol
status: current
verified: 2026-11-08
order: 30
---

# 02 — Riferimenti sul protocollo Sony "MDR" (RFCOMM) per WH-1000XM3 / WH-1000XM4

> Documento di ricerca per la riscrittura clean-room in Rust. Ogni fatto riporta la fonte
> (sigla della tabella **Fonti** + file e riga). Le informazioni che non ho potuto confermare
> da una fonte primaria sono marcate **NON VERIFICATO**. I valori esadecimali derivati da me
> (cioè calcolati applicando l'algoritmo documentato, non copiati da una cattura) sono marcati
> **[derivato]**.
>
> Data della ricerca: 2026-11-03. Commit esaminati: Gadgetbridge `562ae77` (master),
> mos9527/SonyHeadphonesClient `5e50423` (branch `v1-compat`, default), Plutoberth/SonyHeadphonesClient `5620e8e`.

---

## Fonti

| Sigla | Progetto / URL | Licenza (SPDX) | Ultima attività | Modelli dichiarati (rilevanti) |
|---|---|---|---|---|
| **GB** | Gadgetbridge — https://codeberg.org/Freeyourgadget/Gadgetbridge (file sotto `app/src/main/java/nodomain/freeyourgadget/gadgetbridge/{service,devices}/…/sony/headphones/`) | `AGPL-3.0-or-later` (file LICENSE "GNU AFFERO GENERAL PUBLIC LICENSE Version 3"; header dei file: "either version 3 of the License, or (at your option) any later version") | commit `562ae77`, 2026-11-03 | WH-1000XM2/XM3/XM4/XM5/XM6, WF-1000XM3/XM4/XM5, LinkBuds, ecc. Pagina utente https://gadgetbridge.org/gadgets/headphones/sony/: XM3 "Tested with firmware version 4.5.2"; XM4 "Tested with firmware version 2.5.0, but not extensively tested" |
| **MOS** | mos9527/SonyHeadphonesClient — https://github.com/mos9527/SonyHeadphonesClient (libreria `libmdr/`, `libmdr-bt/`, catture in `tests/`) | `MIT` (LICENSE: "Copyright (c) 2026 mos9527, Amr Satrio and other contributors") | commit `5e50423`, 2026-10-31; PR #56 "Support for V1/Legacy devices such as XM3s and XM4s" ancora **aperta** | README r.17-18: WH-1000XM3 (doc dedicata), "WH-1000XM4 (tested by @Amrsatrio)"; molti modelli V2 (XM5/XM6, WF-…) |
| **PLB** | Plutoberth/SonyHeadphonesClient — https://github.com/Plutoberth/SonyHeadphonesClient | `MIT` (Copyright (c) 2020 Nir Harel, Mor Gal, Sem Visscher, jimzrt, guilhermealbm, and other contributors) | commit `5620e8e`, 2025-07-19 | README r.69-70: "WH-1000-XM3: Fully works and supported", "WH-1000-XM4: Partially works, more work is needed" |
| **ANDRO** | andro2157/SonyHeadphonesClient (fork di PLB) — https://github.com/andro2157/SonyHeadphonesClient | `MIT` | 2021-03-23 | stesso stato di PLB (XM4 "Partially works") |
| **MDRP** | AndreasOlofsson/mdr-protocol (documentazione) — https://github.com/AndreasOlofsson/mdr-protocol | **nessun file di licenza** nel repo (tutti i diritti riservati per default) | 2021-10-30 | WF-1000XM3, WH-1000XM3 (README r.17) |
| **AOL** | AndreasOlofsson/libmdr (C, BlueZ) — https://github.com/AndreasOlofsson/libmdr | `GPL-3.0-or-later` | 2021-12-06 | non dichiarato esplicitamente; UUID V1 |
| **PLB#29** | Issue PLB #29 (report utenti XM4) — https://github.com/Plutoberth/SonyHeadphonesClient/issues/29 | (contenuto di issue, non codice) | 2021-2025 | WH-1000XM4 |
| **MOS#56** | PR MOS #56 — https://github.com/mos9527/SonyHeadphonesClient/pull/56 | — | aperta, 2026-08-03 → 2026-10 | XM3/XM4 (V1) |

Percorsi completi per le citazioni:
- GB: `…/service/devices/sony/headphones/protocol/Message.java`, `…/protocol/MessageType.java`, `…/SonyHeadphonesProtocol.java`, `…/SonyHeadphonesSupport.java`, `…/protocol/impl/v1/PayloadTypeV1.java`, `…/protocol/impl/v1/SonyProtocolImplV1.java`, `…/protocol/impl/v2/{PayloadTypeV2,SonyProtocolImplV2}.java`, `…/devices/sony/headphones/coordinators/SonyWH1000XM{3,4}Coordinator.java`, `…/devices/sony/headphones/prefs/*.java`, test in `app/src/test/java/…/sony/headphones/protocol/impl/v{1,2}/SonyProtocolImplV{1,2}Test.java`.
- MOS: `libmdr/include/mdr-c/Base.h`, `libmdr/include/mdr/Command.hpp`, `libmdr/src/Command.cpp`, `libmdr/include/mdr/Protocol.hpp`, `libmdr/include/mdr/ProtocolV1T1.hpp`, `libmdr/src/Headphones.cpp`, `libmdr/src/HeadphonesV1.cpp`, `libmdr/src/Details.hpp`, `client/Client.cpp`, `libmdr-bt/src/Windows/ConnectionWindows.cpp`, `docs/device-support/WH-1000XM3.md`, catture `tests/WH-1000XM3-4.5.2/*.bin`.
- PLB: `Client/Constants.h`, `Client/CommandSerializer.cpp`, `Client/BluetoothWrapper.cpp`, `Client/Headphones.cpp`, `Client/windows/WindowsBluetoothConnector.cpp`.

> Nota di provenienza: MOS#56 dichiara che le definizioni V1/V2 di MOS sono "generated through IDA" (cioè ricavate disassemblando l'app Sony); MDRP dichiara decompilazione dell'APK con dex2jar/fernflower (MDRP `reverse_engineering.md`); PLB#29 riporta che Plutoberth ha decompilato l'app con jadx. Rilevante per la sezione 7.

---

## 1. UUID del servizio RFCOMM (XM3 vs XM4)

**Correzione importante:** l'UUID V1 indicato nella richiesta (`96cc203e-5068-46ad-b32d-396ceebe3928`) **non compare in nessuna fonte** (anche una ricerca web della stringa non dà risultati). Tutte le fonti concordano su un altro valore:

| Nome | UUID esatto (come scritto nella fonte) | Fonti |
|---|---|---|
| V1 | `96CC203E-5068-46ad-B32D-E316F5E069BA` | GB `SonyHeadphonesSupport.java` r.44 (`BTRFCOMM_UUID_V1`); PLB `Constants.h` r.11 (`SERVICE_UUID`); MOS `Base.h` r.50-51 (`MDR_SERVICE_UUID_V1 "96CC203E-5068-46AD-B32D-E316F5E069BA"`); AOL `include/mdr/frameconn.h` r.33; MDRP `README.md` r.17 |
| V2 | `956C7B26-D49A-4BA8-B03F-B17D393CB6E2` | GB `SonyHeadphonesSupport.java` r.45; MOS `Base.h` r.48-49 (`MDR_SERVICE_UUID_V2`) |
| BLE (solo XM5+, non rilevante) | `5B833E20-6BC7-4802-8E9A-723CECA4BD8F` | MOS `Base.h` r.52-53 |

Quale modello usa quale:
- MOS `Base.h` r.48-51, commenti testuali: `// MDR V2, XM5s and newer (SDP service UUID)` e `// MDR V1, XM4s and older (SDP service UUID)`. Quindi **XM3 e XM4 → UUID V1** secondo MOS.
- MOS `docs/device-support/WH-1000XM3.md` r.4: "Tested on firmware 4.5.2, MDR V1 (protocol `0x4010`)".
- GB non decide per modello: `getSupportedService()` (`SonyHeadphonesSupport.java` r.175-202) legge gli UUID SDP del dispositivo e **preferisce V2 se presente**, altrimenti V1; in mancanza di entrambi usa il fallback `preferServiceV2()` del coordinator, che per XM3 e XM4 è `false` (default in `SonyHeadphonesCoordinator.kt` r.144-146; nessun override nei coordinator XM3/XM4) → fallback V1.
- MOS client, modalità automatica: prova prima V2 e poi V1 (`client/Client.cpp` r.1117-1122: `connectionAttempt.services = {MDR_SERVICE_UUID_V2, MDR_SERVICE_UUID_V1};`) e la **famiglia di protocollo è determinata dall'UUID con cui la connessione riesce** (`Client.cpp` r.1066-1074).
- MDRP `README.md` r.17: il canale RFCOMM varia ("channel 9 for my WF-…XM3 and 15 for my WH-…XM3"), quindi **serve la ricerca SDP per UUID**, non un numero di canale fisso.

**NON VERIFICATO:** che un WH-1000XM4 con firmware recente non esponga *anche* l'UUID V2. Se lo esponesse, GB lo sceglierebbe. Da controllare con l'elenco SDP reale (vedi checklist).

---

## 2. Framing sul filo

### 2.1 Struttura (identica in GB, MOS, PLB, MDRP)

```
0x3E | ESCAPE( DataType(1) | Seq(1) | Len(4, big-endian) | Payload(Len) | Checksum(1) ) | 0x3C
```

- Marker di inizio `0x3E` e fine `0x3C`: GB `Message.java` r.48-49 (`MESSAGE_HEADER = 0x3e`, `MESSAGE_TRAILER = 0x3c`); MOS `Command.hpp` r.10-11 (`kStartMarker{62}`, `kEndMarker{60}`); PLB `Constants.h` r.6-7 (`START_MARKER{ 62 }`, `END_MARKER{ 60 }`).
- Lunghezza: 4 byte **big-endian**, conta solo il payload (non escapato): GB `Message.java` r.39 ("Payload Length - 4-byte big endian int") e r.64-69; MOS `Command.cpp` r.80-87 ("Int32BE unescaped size"); MDRP `README.md` r.41-47.
- Il primo byte del payload è l'opcode del comando ("first being the PayloadType", GB `Message.java` r.40).

### 2.2 Escape

- Byte di escape `0x3D`: GB `Message.java` r.50 (`MESSAGE_ESCAPE = 0x3d`); MOS `Command.hpp` r.6 (`kEscapedByteSentry = 0x3D`); PLB `CommandSerializer.cpp` r.3 (`ESCAPED_BYTE_SENTRY = 61`).
- Regola: i byte `0x3C`, `0x3D`, `0x3E` vengono sostituiti da `0x3D` seguito dal byte **AND `0xEF`** (cioè con il bit 4 azzerato); in decodifica si fa **OR `0x10`**. GB `Message.java` r.51 (`MESSAGE_ESCAPE_MASK = (byte) 0b11101111`), r.156-174 (`b & MESSAGE_ESCAPE_MASK`), r.176-192 (`bytes[i] | ~MESSAGE_ESCAPE_MASK`); MDRP `README.md` r.26-28.
- Tabella esplicita: `0x3C→3D 2C`, `0x3D→3D 2D`, `0x3E→3D 2E` — MOS `Command.hpp` r.7-9; PLB `CommandSerializer.cpp` r.4-6; MDRP `README.md` r.31-35.
- L'escape copre **tutto** ciò che sta tra i marker, **checksum incluso**: GB `Message.java` r.145-146; MOS `Command.cpp` r.89-93.
- MOS (`Command.cpp` r.51-64) e PLB rifiutano come errore una sequenza `3D xx` con `xx` diverso da 2C/2D/2E; GB invece fa OR 0x10 su qualsiasi byte.

### 2.3 Tipi di dato (byte 0 del frame)

Valori esatti (MOS `Protocol.hpp` r.37-56; PLB `Constants.h` `enum class DATA_TYPE`; MDRP `README.md` r.53-70):

| Valore | Nome |
|---|---|
| `0x00` | DATA |
| `0x01` | **ACK** |
| `0x02` | DATA_MC_NO1 |
| `0x09` | DATA_ICD |
| `0x0A` | DATA_EV |
| `0x0C` | **DATA_MDR** (GB: `COMMAND_1`) |
| `0x0D` | DATA_COMMON |
| `0x0E` | **DATA_MDR_NO2** (GB: `COMMAND_2`) |
| `0x10` | SHOT |
| `0x12` | SHOT_MC_NO1 |
| `0x19` | SHOT_ICD |
| `0x1A` | SHOT_EV |
| `0x1C` | SHOT_MDR |
| `0x1D` | SHOT_COMMON |
| `0x1E` | SHOT_MDR_NO2 |
| `0x2D` | LARGE_DATA_COMMON |

GB usa solo `ACK(0x01)`, `COMMAND_1(0x0c)`, `COMMAND_2(0x0e)` (`MessageType.java` r.19-24). MOS chiama `0x0C` "Table 1" e `0x0E` "Table 2" (`Headphones.cpp` r.189-212). Per XM3/XM4 servono in pratica solo `0x01`, `0x0C`, `0x0E`.

### 2.4 Numero di sequenza

- 1 byte, alterna `0x00`/`0x01`. MDRP `README.md` r.72-74: per ogni frame non-ACK l'altra parte risponde con un ACK a payload vuoto e "the sequence ID is set to the opposite of the received frame"; se un frame non riceve ACK va **ritrasmesso con lo stesso seq** ("sequence ID is only changed when an Ack is received").
- ACK = `1 - seq_ricevuto`: GB `SonyHeadphonesProtocol.java` r.426-428 (`new Message(MessageType.ACK, (byte) (1 - sequenceNumber), new byte[0])`); MOS `Headphones.cpp` r.310-313 (`SendCommandImpl({}, MDRDataType::ACK, 1 - seq)`).
- Seq dei comandi dell'host: parte da 0 (GB `SonyHeadphonesProtocol.java` r.75); quando arriva un ACK dal device, il prossimo comando usa il seq contenuto nell'ACK (GB r.100-109: `sequenceNumber = message.getSequenceNumber()`; MOS `Headphones.cpp` r.315-324: atteso `1 - mSeqNumber`, poi `mSeqNumber = expected`; PLB `BluetoothWrapper.cpp` r.95-96).
- GB scarta come "Unexpected ACK" un ACK che ha lo stesso seq di quello corrente (r.101-104); MOS lo logga come "Stale ACK" e lo ignora (r.317-321).
- I due versi hanno sequenze **indipendenti**: nella cattura XM3 di MOS la prima risposta del device (`CONNECT_RET_PROTOCOL_INFO`) ha seq `00` (file `tests/WH-1000XM3-4.5.2/…-000001-rx…seq-00…PROTOCOL_INFO-01.bin`).

### 2.5 Checksum

- Somma a 8 bit (modulo 256) di **tutti i byte non escapati tra i marker, escluso il checksum stesso**: DataType + Seq + 4 byte di lunghezza + payload. GB `Message.java` r.41 ("1-byte sum, excluding header"), r.142 (`calcChecksum(message, 0, message.length)` sul buffer `type|seq|len|payload`), r.194-200; MOS `Command.cpp` r.6-12 e r.89, verifica r.122-128 ("Checksum incl. type,seq,data"); MDRP `README.md` r.115.
- **Bug noto in PLB** (utile come pitfall): `unpackBtMessage` calcola il checksum sui byte *ancora escapati* (`CommandSerializer.cpp` r.139-152 usa `src` invece di `unescaped`). Questo spiega gli errori "Invalid checksum!" segnalati con l'XM4 in PLB#29 (commenti del 3 e 8 marzo 2021). Il checksum va calcolato **dopo** l'unescape.

### 2.6 Esempi completi

Catture reali WH-1000XM3 fw 4.5.2 (MOS `tests/WH-1000XM3-4.5.2/`, letti con `xxd`):

| Significato | Frame |
|---|---|
| ACK dal device (seq 1) | `3e 01 01 00 00 00 00 02 3c` |
| `CONNECT_RET_PROTOCOL_INFO` (payload `01 00 40 10`) | `3e 0c 00 00 00 00 04 01 00 40 10 61 3c` |
| `CONNECT_RET_DEVICE_INFO` modello ("WH-1000XM3") | `3e 0c 00 00 00 00 0d 05 01 0a 57 48 2d 31 30 30 30 58 4d 33 8e 3c` |
| `CONNECT_RET_DEVICE_INFO` firmware ("4.5.2") | `3e 0c 01 00 00 00 08 05 02 05 34 2e 35 2e 32 18 3c` |
| `NCASM_RET_PARAM` (NC attivo) | `3e 0c 00 00 00 00 08 67 02 01 02 02 01 00 00 83 3c` |

Verifica del checksum sull'esempio: `0c+00+00+00+00+04+01+00+40+10 = 0x61` ✔ (ho ricalcolato tutti i frame qui sopra con l'algoritmo del §2.5 e coincidono).

Frame host→device dai test GB (`SonyProtocolImplV2Test.java`), utili come vettori di test del framing:
- r.218: `3e:0c:00:00:00:00:02:f6:0d:11:3c`
- r.278: `3e:0c:01:00:00:00:04:58:00:00:00:69:3c`

Frame host **[derivato]** (calcolati da me, da usare come test, confermare con le catture):

| Significato | Frame |
|---|---|
| Init `CONNECT_GET_PROTOCOL_INFO`, seq 0, payload `00 00` | `3e 0c 00 00 00 00 02 00 00 0e 3c` |
| ACK host per un frame ricevuto con seq 0 | `3e 01 01 00 00 00 00 02 3c` |
| ACK host per un frame ricevuto con seq 1 | `3e 01 00 00 00 00 00 01 3c` |
| Esempio di escape: payload `58 01 3c` | `3e 0c 00 00 00 00 03 58 01 3d 2c a4 3c` |

### 2.7 Riassemblaggio dello stream

- Un frame può arrivare spezzato su più letture RFCOMM, e più frame possono arrivare in una sola lettura: trattare l'input come stream (MDRP `README.md` r.24). GB accumula in un buffer da 2048 byte e cerca `0x3E…0x3C` (`SonyHeadphonesSupport.java` r.51, r.129-173). MOS scarta i byte del frame se checksum/lunghezza non tornano (`Headphones.cpp` r.262-269).
- PLB lancia eccezione se il pacchetto supera 2048 byte (`CommandSerializer.cpp` r.127-131, `MAX_BLUETOOTH_MESSAGE_SIZE = 2048`). Alcune risposte (p.es. JSON `0xC9` in GB) possono essere lunghe: non fissare limiti troppo bassi.

---

## 3. Gestione degli ACK

- **L'host deve inviare un ACK per ogni frame DATA_MDR/DATA_MDR_NO2 ricevuto.** GB lo fa sempre, prima di interpretare il payload (`SonyHeadphonesProtocol.java` r.181-185); MOS idem (`Headphones.cpp` r.189-190 e r.203-204 `SendACK(seq)`); MDRP `README.md` r.74 ("The other party should reply to each non-Ack frame with an Ack").
- **Cosa succede se non si fa ACK:** secondo MDRP il mittente ritrasmette con lo stesso seq (r.74, diagramma r.94-104 con "timeout"). Che il device **si disconnetta** in assenza di ACK: **NON VERIFICATO** (nessuna fonte lo afferma). PLB non invia mai ACK all'host-side (in `BluetoothWrapper.cpp` non c'è alcun invio di ACK; legge solo l'ACK della propria richiesta, r.25-34 e r.60-97) eppure i SET funzionano sull'XM3; ma con l'XM4 PLB#29 riporta blocchi ("Sending command…" per circa 60 s, 15 gen 2025) ed errori. Ipotesi plausibile, **NON VERIFICATA**: il device ritrasmette le notifiche non confermate e/o accoda.
- **Una richiesta alla volta (stop-and-wait):** GB invia la richiesta successiva della coda solo quando arriva l'ACK della precedente (commento "Request queue, sent every ACK, as we can't send them all at once", `SonyHeadphonesProtocol.java` r.77-80; `handleAck()` r.464-483; `SonyHeadphonesSupport.java` r.97-103). MOS: ogni comando aspetta l'ACK con **timeout 1000 ms e fino a 10 tentativi**, ritrasmettendo con lo stesso seq (`Details.hpp` r.418-419 `mACKRetryCount = 10u`, `mACKRetryTimeoutMS = 1000u`; macro `SendCommandACK` r.522-541, nota "Retries re-send with the same mSeqNumber"). Timeout di default per le risposte: 5000 ms (`Details.hpp` r.420).
- **Ordine RET/ACK sull'XM3:** "The device answers a request before it acknowledges it. Every `*_RET_*` reply in the captures arrives ahead of the ACK for the `*_GET_*` request it answers (notifications after a `*_SET_*` come after the ACK)" — MOS `docs/device-support/WH-1000XM3.md` r.43-48. Il parser deve quindi accettare la risposta prima dell'ACK; se si aspetta prima l'ACK e si scarta ciò che arriva prima, si perde la risposta all'init (stesso documento, r.45-48).
- MOS#56 (commit 4061cf3) descrive una race: un RET/NTFY non atteso veniva memorizzato e "svegliava" in anticipo l'attesa successiva; soluzione: correlare le attese con un contatore di pacchetti.
- Se il device "ignora" la prima richiesta: GB ritrasmette l'init fino a 2 volte ogni 1250 ms e poi rinuncia (`SonyHeadphonesSupport.java` r.53-79, r.204-208, commento "Sometimes the headphones will ignore the first init request, so we retry a few times").

---

## 4. Handshake di inizializzazione

### 4.1 Primo comando

`CONNECT_GET_PROTOCOL_INFO` = opcode `0x00`, payload completo **`00 00`** (opcode + "fixed value" `0x00`), DataType `0x0C`, seq 0.
- GB `SonyHeadphonesProtocol.java` r.430-440 (`encodeInit()`: `COMMAND_1`, payload `{0x00, 0x00}`); MOS `ProtocolV1T1.hpp` r.214 (`CONNECT_GET_PROTOCOL_INFO = 0x00`) e struct `GetProtocolInfo` r.1562-1569 (`command`, `CommonCapabilityInquiredType type{FIXED_VALUE}` = `0x00`); MDRP `packets/CONNECT_XXX_PROTOCOL_INFO.md` r.7-11 ("This is the first packet sent by the app").
- MDRP `MDR_packets.md` r.7: va inviato per primo "even if the information is not useful to the controller as some functions don't seem to work unless the device has received this packet first".
- Frame **[derivato]**: `3e 0c 00 00 00 00 02 00 00 0e 3c`.

### 4.2 Risposta e selezione V1/V2

`CONNECT_RET_PROTOCOL_INFO` = opcode `0x01`.
- **V1 (4 byte):** `01 | 00 (fixed) | versione Int16 BE` — MOS `ProtocolV1T1.hpp` r.3108-3116 (`RetProtocolInfo`: `command`, `type`, `Int16BE protocolVersion`); MDRP `CONNECT_XXX_PROTOCOL_INFO.md` r.17-22.
- Valore **XM3 reale**: `01 00 40 10` → versione `0x4010` (cattura MOS, §2.6; GB `SonyHeadphonesProtocol.java` r.134 commento `// WH-1000XM3:      01:00:40:10`; MOS doc XM3 r.4).
- GB sceglie l'implementazione **dalla lunghezza della risposta**: 4 byte → `v1`, 8 byte → `v2` (`SonyHeadphonesProtocol.java` r.120-171). Esempi V2 riportati: WF-1000XM4 `01:00:01:00:00:00:00:00`, WH-1000XM5 `01:00:03:00:00:00:00:00` (r.139-142). Nota: **WF**-1000XM4 (auricolari) è V2; **WH**-1000XM4 non compare nei commenti.
- MOS invece fissa la famiglia dall'UUID usato (§1) e verifica che la risposta abbia la dimensione della struct V1 (`HeadphonesV1.cpp` r.792-808: "MDR V1 protocol info size is not valid").
- **WH-1000XM4: V1 secondo MOS** (`Base.h` r.50 "XM4s and older"; README r.18-22 "v1 (XM4 and older) devices"). In GB l'indizio è indiretto: le codifiche Speak-to-Chat V1 (`0xF8 05 …`, `0xFC 05 …`, `SonyProtocolImplV1.java` r.164-211) sono usate solo da dispositivi V1 con capacità SpeakToChat, e tra i coordinator con SpeakToChat l'unico non-V2 è WH-1000XM4. **Versione esatta del protocollo XM4 (`0x40xx`?) e lunghezza della risposta: NON VERIFICATE** → catturare.
- MDRP `CONNECT_XXX_PROTOCOL_INFO.md` r.28-43: versioni note nell'app `0x1000, 0x2000, 0x3000, 0x4000, 0x4010, 0x5000, 0x6000, 0x7000, 0x7010`, con soglie di capacità (p.es. `isInstructionGuideSupport: >= 0x5000`).

### 4.3 Sequenza successiva

**MOS (V1, `HeadphonesV1.cpp` r.20-40 e seguenti):**
1. `GetProtocolInfo` (attende ACK **e** risposta).
2. `CONNECT_GET_CAPABILITY_INFO` (`0x02`).
3. `CONNECT_GET_DEVICE_INFO` (`0x04`) con inquired type `MODEL_NAME 0x01`, `FW_VERSION 0x02`, `SERIES_AND_COLOR_INFO 0x03`, `INSTRUCTION_GUIDE 0x04` (enum `DeviceInfoInquiredType`, `ProtocolV1T1.hpp` r.409-416).
4. `CONNECT_GET_SUPPORT_FUNCTION` (`0x06`, payload `06 00`) → `0x07` con l'elenco delle funzioni supportate (struct `RetSupportFunction` r.3151-3160: `07 | type | count | FunctionType…`).
5. Per ogni funzione supportata: GET_CAPABILITY, poi GET_STATUS / GET_PARAM (EQ, NC/ASM, play, audio, system, general settings, voice guidance in Table 2…), `HeadphonesV1.cpp` r.42-330.

Catture XM3: `CONNECT_RET_SUPPORT_FUNCTION` = `07 00 13 11 c1 12 13 14 30 71 62 81 42 41 51 a1 e1 e2 d1 d2 f4 39` (19 funzioni); MOS doc XM3 r.75-88 elenca: BATTERY_LEVEL, ACTION_LOG_NOTIFIER, UPSCALING_INDICATOR, CODEC_INDICATOR, BLE_SETUP, FW_UPDATE, AUTO_NC_ASM, NOISE_CANCELLING_AND_AMBIENT_SOUND_MODE, NC_OPTIMIZER, SOUND_POSITION, VPT, PRESET_EQ, PLAYBACK_CONTROLLER, CONNECTION_MODE, UPSCALING, GENERAL_SETTING1, GENERAL_SETTING2, AUTO_POWER_OFF, VOICE_GUIDANCE.

**GB (V1, `SonyProtocolImplV1.java` r.767-818):** dopo l'init chiede firmware (`04 02`), codec (`18 00`) e poi solo i GET relativi alle capacità **dichiarate staticamente** nel coordinator del modello (non usa `GET_SUPPORT_FUNCTION`). Capacità dichiarate:
- XM3 (`SonyWH1000XM3Coordinator.java` r.44-61): BatterySingle, AmbientSoundControl, WindNoiseReduction, AncOptimizer, AudioSettingsOnlyOnSbcCodec, AudioLDAC, ButtonFunctionNcAmbient, EqualizerWithCustomBands, SoundPosition, SurroundMode, AudioUpsampling, TouchSensorSingle, AutomaticPowerOffByTime, VoiceNotifications, Volume.
- XM4 (`SonyWH1000XM4Coordinator.java` r.46-62): BatterySingle, AmbientSoundControl, WindNoiseReduction, SpeakToChatEnabled, SpeakToChatConfig, SpeakToChatFocusOnVoice, AncOptimizer, EqualizerWithCustomBands, AudioUpsampling, TouchSensorSingle, PauseWhenTakenOff, AutomaticPowerOffWhenTakenOff, VoiceNotifications (con TODO per il tasto CUSTOM e la connessione a due dispositivi).

Raccomandazione per il nostro client (deduzione, non da fonte): seguire lo schema MOS (protocol info → device info → support function) e abilitare le funzioni in base a `0x07`, così XM3 e XM4 non vanno distinti a mano.

---

## 5. Tabella dei comandi (Table 1 = `0x0C`, salvo dove indicato)

Nomi ufficiali da MOS `ProtocolV1T1.hpp` r.212-348 (enum `Command`), nomi GB da `PayloadTypeV1.java` r.21-101. Gli offset dei payload qui sotto includono l'opcode come byte 0.

### 5.1 Opcode principali V1

| Funzione | GET | RET | SET | NTFY | Fonti |
|---|---|---|---|---|---|
| Protocol info | `0x00` | `0x01` | — | — | MOS r.214-215; GB r.22-23 |
| Capability info | `0x02` | `0x03` | — | — | MOS r.216-217 |
| Device info (modello/fw/colore) | `0x04` | `0x05` | — | — | MOS r.218-219; GB `FW_VERSION_REQUEST/REPLY` r.25-26 |
| Support function | `0x06` | `0x07` | — | — | MOS r.220-221; GB `INIT_2_*` r.28-29 |
| Batteria | `0x10` | `0x11` | — | `0x13` | MOS r.222-224; GB r.31-33 |
| Codec audio | `0x18` | `0x19` | — | `0x1B` | MOS r.228-230; GB r.35-37 |
| Spegnimento | — | — | `0x22` | — | MOS r.233; GB r.39 |
| VPT / Sound position | `0x46` | `0x47` | `0x48` | `0x49` | MOS r.250-253; GB r.41-44 |
| Equalizzatore | `0x56` | `0x57` | `0x58` | `0x59` | MOS r.259-262; GB r.46-49 |
| NC / Ambient | `0x66` | `0x67` | `0x68` | `0x69` | MOS r.270-273; GB r.51-54 |
| NC optimizer | `0x86` | `0x87` | `0x84` (start) | `0x85`/`0x89` | MOS r.281-285; GB r.61-66 |
| Volume / play | `0xA6` | `0xA7` | `0xA8` | `0xA9` | MOS r.297-300; GB r.56-59 |
| General setting (touch, tasto NC/AMB) | `0xD6` | `0xD7` | `0xD8` | `0xD9` | MOS r.321-324 (`GENERAL_SETTING_NTNY_PARAM` è il refuso originale); GB `TOUCH_SENSOR_*` r.68-71 |
| Audio (DSEE/upscaling, LDAC/qualità) | `0xE6` | `0xE7` | `0xE8` | `0xE9` | MOS r.330-333; GB r.73-76 |
| System param (auto-off, pause-on-remove, S2C on/off) | `0xF6` | `0xF7` | `0xF8` | `0xF9` | MOS r.339-342; GB `AUTOMATIC_POWER_OFF_BUTTON_MODE_*` r.78-81 |
| System extended param (config S2C, auto-off per MOS) | `0xFA` | `0xFB` | `0xFC` | `0xFD` | MOS r.343-346; GB `SPEAK_TO_CHAT_CONFIG_*` r.83-86 |
| Voice guidance (Table 2, `0x0E`) | `0x46` | `0x47` | `0x48` | `0x49` | GB `VOICE_NOTIFICATIONS_*` r.96-99 |

Ogni categoria ha anche `*_GET_CAPABILITY`/`*_RET_CAPABILITY` (`0x?0/0x?1`) e `*_GET_STATUS`/`*_RET_STATUS`/`*_NTFY_STATUS` (`0x?2/0x?3/0x?5`) — MOS r.245-346.

### 5.2 Batteria (over-ear: singola)

- GET: `10 00` (inquired type `BATTERY = 0x00`). GB `getBattery` r.236-244 + `encodeBatteryType` r.1461-1473 (`SINGLE → 0x00`, `DUAL → 0x01`, `CASE → 0x02`); MOS `BatteryInquiredType` r.198-204 (`BATTERY 0x00`, `LEFT_RIGHT_BATTERY 0x01`, `CRADLE_BATTERY 0x02`). Test GB V1 r.195: `BatteryType.SINGLE → "10:00"`.
- RET `0x11` / NTFY `0x13`: `11|13 | 00 | livello % | stato_ricarica`. GB `handleBattery` r.1089-1098 (`level = payload[2]`, `payload[3] == 1` → in carica); MOS struct `RetBatteryLevelBatteryParam` r.2717-2729 e `NotifyBatteryLevelBatteryParam` r.1938-1950; `BatteryChargingStatus` r.191-196: `NOT_CHARGING 0x00`, `CHARGING 0x01`, `UNKNOWN 0xF0`.
- Cattura XM3: `11 00 64 00` = 100 %, non in carica (file MOS `…COMMON_RET_CAPABILITY-11.bin`; il nome file usa un'etichetta errata, ma l'opcode `0x11` in Table 1 è `COMMON_RET_BATTERY_LEVEL`).
- Discrepanza: MDRP `COMMON_XXX_BATTERY_LEVEL.md` r.76-80 indica `LEFT_RIGHT = 0x02`, `CRADLE = 0x03`; GB e MOS concordano su `0x01`/`0x02`. Irrilevante per XM3/XM4 (usano `0x00`).
- Le NTFY arrivano solo dopo il GET corrispondente (MDRP `MDR_packets.md` r.3) — **NON VERIFICATO** sul campo per la batteria.

### 5.3 Noise cancelling / Ambient sound (XM3 e XM4, V1)

Struttura SET `0x68` / RET `0x67` / NTFY `0x69` (8 byte), MOS `SetNcAsmParamNcAsmParam` r.3743-3760 e `NcAsmParam` r.1836-1849:

| Byte | Campo (MOS) | Valori |
|---|---|---|
| 0 | opcode | `68` / `67` / `69` |
| 1 | `NcAsmInquiredType` | `0x02` = NOISE_CANCELLING_AND_AMBIENT_SOUND_MODE (r.627-634) |
| 2 | `NcAsmEffect` | `0x00` OFF, `0x01` ON, `0x10` ADJUSTMENT_IN_PROGRESS, `0x11` ADJUSTMENT_COMPLETION (r.618-625) |
| 3 | `NcAsmSettingType` | `0x00` ON_OFF, `0x01` LEVEL_ADJUSTMENT, `0x02` DUAL_SINGLE_OFF (r.636-641) |
| 4 | valore NC | con DUAL_SINGLE_OFF: `0x00` OFF, `0x01` SINGLE, `0x02` DUAL (r.643-649) |
| 5 | `AsmSettingType` | `0x00` ON_OFF, `0x01` LEVEL_ADJUSTMENT (r.64-68) |
| 6 | `AsmId` | `0x00` NORMAL, `0x01` VOICE (= focus on voice) (r.50-55) |
| 7 | livello ambient | 0..20 |

Semantica dei modi (GB `setAmbientSoundControl` r.96-149 e `handleAmbientSoundControl` r.820-877):
- byte 2: `0x00` = tutto spento; altrimenti GB invia `0x11` e in ricezione accetta `0x01`.
- byte 3 = `0x02` se il modello supporta wind reduction (XM3 e XM4 lo dichiarano in GB), allora byte 4: `2` = NC, `1` = wind noise reduction, `0` = ambient sound. Senza wind: byte 3 = `0x00` e byte 4: `1` = NC, `0` = ambient.
- byte 5 = `0x01` costante ("// ?" in GB r.134); byte 6 = focus voice; byte 7 = livello (solo in ambient/off, `0` in NC/wind). GB rifiuta livelli fuori da 0..20 (r.863-867).
- Vettori di test GB V1 (con wind, `SonyProtocolImplV1Test.java` r.102-116): NC → `68:02:11:02:02:01:00:00`; ambient livello 15 + voice → `68:02:11:02:00:01:01:0F`; wind → `68:02:11:02:01:01:00:00`; off → `68:02:00:02:00:01:00:00`. GET: `66 02` (test r.75).
- Catture XM3 reali: `67 02 01 02 02 01 00 00` (NC attivo); NTFY dopo pressione del tasto: `69 02 00 02 02 01 00 00` e `69 02 01 02 02 01 00 00` (file `…NCASM_NTFY_PARAM-69.bin`).
- Capacità XM3 `61 02 02 00 01 02 00 14 01 14`: interpretazione (mia, da struct MOS `SetNcAsmParamRetNcAsmCapability_NcAsmCapability` r.5223-5235): tipo `02`, nc setting `02` (dual/single/off), …, 2 voci ASM (`00`=NORMAL, `01`=VOICE) con `0x14` = 20 passi. **Interpretazione byte per byte NON VERIFICATA**.

Divergenze tra implementazioni (da risolvere con le catture dell'app ufficiale):
- PLB e MOS inviano byte 3 = `0x01` (LEVEL_ADJUSTMENT) anziché `0x02` (PLB `Headphones.cpp` r.84-94, `CommandSerializer.cpp` r.174-186; MOS `HeadphonesV1.cpp` r.553-561); GB invia `0x02`; il device XM3 **risponde** con `0x02`.
- PLB invia `ADJUSTMENT_COMPLETION (0x11)` quando abilitato (`Headphones.cpp` r.84); GB idem; MOS usa il valore di `mNcAsmEffect` dalla UI (può essere `0x10` durante il trascinamento dello slider). Il commento GB V2 r.91 "0x00 while dragging the slider?" suggerisce un comportamento simile nell'app ufficiale.
- PLB limita il livello a 19 (`MAX_STEPS_WH_1000_XM3 = 19`, `CommandSerializer.cpp` r.7); GB e MOS a 20.
- Mappatura NC/wind in PLB: livello 0 → DUAL, 1 → SINGLE (`getDualSingleForAsmLevel`, r.156-172), cioè codifica NC/wind nello slider; MOS fa una mappatura analoga (`V1NcValueForAmbientLevel`, `HeadphonesV1.cpp` r.10-17).

**XM3 vs XM4:** nessuna fonte mostra una codifica NC/ASM diversa tra XM3 e XM4 in V1; GB usa lo stesso codice per entrambi. **NON VERIFICATO** che l'XM4 (che nell'app ufficiale non ha la modalità "wind noise reduction" separata — informazione **NON VERIFICATA**) accetti `byte4 = 1`. Da catturare.

### 5.4 Pressione del tasto fisico NC/AMB

- XM3: arriva `NCASM_NTFY_PARAM (0x69)`, preceduto da un `PLAY_NTFY_PARAM (0xA9)` con il volume corrente — MOS doc XM3 r.63-65; catture: `a9 01 20 18` poi `69 02 01 02 02 01 00 00`.
- GB tratta `0x69` con lo stesso parser di `0x67` (`SonyProtocolImplV1.java` r.732-734).
- XM4: **NON VERIFICATO** (atteso uguale).

### 5.5 Equalizzatore + Clear Bass

- GET `56 01` (inquired `PRESET_EQ = 0x01`) — GB r.482-490; MOS `EqEbbInquiredType` r.448-455.
- RET `0x57` / NTFY `0x59` (10 byte): `57 | 01 | preset | 06 | clearBass+10 | b1+10 … b5+10` — GB `handleEqualizer` r.1154-1184 (lunghezza 10, preset = byte 2, clear bass = byte 4 − 10, 5 bande = byte 5..9 − 10). MOS: `SetEqEbbParamEqParam` r.3640-3650 (`command | type | presetId | MDRPodArray<UInt8> bandSteps` → byte 3 = numero di valori).
- Cattura XM3: `57 01 00 06 0a 0a 0a 0a 0a 0a` (preset OFF, tutto a 0); NTFY dopo cambio: `59 01 00 06 0a 0a 0a 0a 0a 0a`.
- SET preset: `58 01 <preset> 00` (GB r.493-503). SET bande custom: `58 01 FF 06 <bass+10> <5 bande+10>` (GB r.506-523; MOS r.598-610 usa `presetId = UNSPECIFIED (0xFF)` e clamp −10..+10).
- Ordine bande in GB: 400, 1000, 2500, 6300, 16000 Hz (`SonyHeadphonesProtocol.java` r.233-238). Quindi **6 valori = Clear Bass + 5 bande**, scala −10..+10 con offset +10.
- Preset (GB `prefs/EqualizerPreset.java` r.32-43; MOS `EqPresetId` r.457-…): `00` OFF, `10` BRIGHT, `11` EXCITED, `12` MELLOW, `13` RELAXED, `14` VOCAL, `15` TREBLE_BOOST, `16` BASS_BOOST, `17` SPEECH, `A0` MANUAL, `A1` CUSTOM_1, `A2` CUSTOM_2.
- Capacità XM3 `51 01 06 15 0c 00 00 10 00 11 00 … a2 00`: 6 bande, `0x15` = 21 passi, `0x0c` = 12 preset (gli ID sopra). Interpretazione da struct MOS `SetEqEbbParamRetEqEbbCapability_EqCapability` r.5167-5177 (`type | band | step | presetList`).
- Pitfall XM3 (MOS doc r.69-73): cambiare il preset ha generato `ALERT_NTFY_PARAM` "DISCONNECT_CAUSED_BY_CONNECTION_MODE_CHANGE" con scelta sì/no; il client ha risposto NEGATIVE e il preset è stato applicato senza disconnessione. GB ha la capacità `AudioSettingsOnlyOnSbcCodec` per l'XM3 (coordinator r.50): le impostazioni audio valgono solo con codec SBC (interpretazione dal nome, **NON VERIFICATA**).
- **XM3 vs XM4:** GB usa lo stesso formato a 10 byte per entrambi; numero di bande / passi dell'XM4: **NON VERIFICATO** (leggere `0x51`).

### 5.6 Informazioni dispositivo

- GET `04 <tipo>`: `01` MODEL_NAME, `02` FW_VERSION, `03` SERIES_AND_COLOR_INFO, `04` INSTRUCTION_GUIDE (MOS r.409-416). RET `05 <tipo> <len> <ASCII>` (GB `handleFirmwareVersion` r.1186-1217: byte 2 = lunghezza, testo da byte 3).
- XM3: modello `05 01 0a "WH-1000XM3"`, firmware `05 02 05 "4.5.2"`, serie/colore `05 03 30 01` (catture MOS).
- XM3 non risponde a `INSTRUCTION_GUIDE`, `COMMON_GET_UPSCALING_EFFECT` e ad alcuni `UPDT_GET_PARAM` (fa ACK ma non risponde) — MOS doc XM3 r.66-68. Quindi **non bloccare l'init** in attesa di risposte facoltative.

### 5.7 Spegnimento automatico

- GB (V1): GET `F6 04`, SET `F8 04 01 <a> <b>`, RET `F7 04 01 <a> <b>` (r.330-353, `handleAutomaticPowerOff` r.986-1010). Codici (`prefs/AutomaticPowerOff.java` r.32-37): OFF `11 00`, 5 min `00 00`, 30 min `01 01`, 1 h `02 02`, 3 h `03 03`, WHEN_TAKEN_OFF `10 00`.
- MOS: `AutoPowerOffElementId` r.158-166 (`00` 5 min, `01` 30 min, `02` 60 min, `03` 180 min, `10` when removed, `11` disable), `AutoPowerOffParameterType 0x01` (r.168-172); MOS invia con **`SYSTEM_SET_EXTENDED_PARAM (0xFC)`** (`HeadphonesV1.cpp` r.638-647; doc XM3 r.57-62: "The device acknowledges every value but sends no notification"; non esiste 15 min in V1).
- Cattura XM3 RET: `f7 04 01 00 00` (5 min).
- **Discrepanza da verificare:** GB scrive con `0xF8`, MOS con `0xFC`. Entrambi dichiarano XM3 funzionante. Catturare quale usa l'app ufficiale per XM3 e XM4.
- XM4 in GB: `AutomaticPowerOffWhenTakenOff` (invece di "ByTime" dell'XM3) + `PauseWhenTakenOff` (`F8 03 00 <0/1>`, GB r.458-479).

### 5.8 Touch sensor / tasto NC-AMB (General settings)

- GB: GET `D6 D2`, SET `D8 D2 01 <0/1>` (touch panel); GET `D6 D1`, SET `D8 D1 02 <modo>` per la funzione del tasto NC/AMB (`00` switch ambient, `01` Google Assistant, `02` Alexa; `prefs/ButtonFunctionNcAmbient.java` r.31-33) — `SonyProtocolImplV1.java` r.293-314, r.572-593, parser r.1327-1363. Dopo aver cambiato la funzione del tasto GB invia anche un **reboot** `98 01 02 01` (`SonyHeadphonesProtocol.java` r.244-250; `SonyProtocolImplV1.java` r.668-678).
- MOS doc XM3 r.53-56: `GENERAL_SETTING2` = `TOUCH_PANEL_SETTING` (booleano, confermato con `0xD9`), `GENERAL_SETTING1` = `ASSIGNABLE_KEY_SETTING` (lista). Catture: `d7 d2 01 01` (touch on), `d7 d1 02 00` (tasto = NC/AMB).
- Nota: MOS associa `0xD1`/`0xD2` ai *tipi* GENERAL_SETTING1/2; il significato concreto (touch vs tasto) va letto dalla capability `0xD1` (`GENERAL_SETTING_RET_CAPABILITY`). Su XM4 la mappatura potrebbe differire: **NON VERIFICATO**.

### 5.9 Speak-to-Chat (solo XM4)

- On/off (GB V1): GET `F6 05`; SET `F8 05 01 <0/1>` (test GB V1 r.128-129: `f8:05:01:00`, `f8:05:01:01`; codice r.164-185). RET con byte 2 = `0x01` → stato; GB nota che dopo un SET arriva un `0x02` in byte 2 (r.1018-1021).
- Configurazione (GB V1): GET `FA 05`; SET `FC 05 00 <sensibilità> <focus voice 0/1> <timeout>`; RET `FB 05 00 …` (6 byte) — r.188-211, r.1365-1402. Sensibilità (`prefs/SpeakToChatConfig.java` r.34-36): `00` AUTO, `01` HIGH, `02` LOW. Timeout (r.69-72): `00` SHORT, `01` STANDARD, `02` LONG, `03` OFF.
- MOS: `SystemInquiredType.SMART_TALKING_MODE = 0x05` (r.895-904) e la richiesta di capability durante l'init (`HeadphonesV1.cpp` r.110-115); il SET non risulta implementato in V1 (doc XM3: "Speak to Chat ❌" per XM3, ovviamente assente sull'XM3).
- La pagina utente GB dice per XM4: "Speak-to-chat settings (only enable/disable is implemented)", in apparente contrasto con il codice che ha anche la configurazione. **Codifica XM4 NON VERIFICATA da cattura reale.**

### 5.10 Altri comandi utili

- Codec: GET `18 00` → RET `19 00 <codec>`: `01` SBC, `02` AAC, `10` LDAC, `20` aptX, `21` aptX HD (GB r.225-233, `params/AudioCodec.java` r.20-25).
- Volume: GET `A6 01 20`, SET `A8 01 20 <vol>` (GB r.687-709); NTFY XM3 `a9 01 20 <0..30>` (catture; MOS clamp 0..30, `HeadphonesV1.cpp` r.568-576).
- DSEE/upscaling: GET `E6 02`, SET `E8 02 00 <0/1>` (GB r.258-266, r.317-327). LDAC/qualità: GET `E6 01`, SET `E8 01 00 <inverso>` (GB r.269-290, nota: "enabled ? 0x00 : 0x01").
- Spegnimento: `22 00 01` (GB r.656-665).
- Voice guidance (Table 2 `0x0E`): GET `46 …`, SET `48 …` (GB r.596-620; MOS doc XM3 r.61-62: confermato con NTFY, nessun volume voce sull'XM3).

---

## 6. Pitfall noti

1. **UUID V1 corretto** è `96CC203E-5068-46AD-B32D-E316F5E069BA` (§1), non quello nella richiesta.
2. **Usare SDP per trovare il canale RFCOMM** (varia per modello/firmware, MDRP r.17).
3. **Checksum sui byte non escapati** (bug PLB, §2.5) e escape anche del checksum.
4. **Risposta prima dell'ACK** sull'XM3 (MOS doc XM3 r.43-48): gestire RET che arriva prima dell'ACK della richiesta.
5. **Una richiesta in volo per volta**, ritrasmissione con stesso seq dopo ~1 s, fino a 10 volte (MOS `Details.hpp` r.418-419); GB rinvia l'init 2 volte ogni 1,25 s.
6. **ACK a ogni frame dati ricevuto**, anche notifiche non richieste. Disconnessione per ACK mancante: **NON VERIFICATO**.
7. **Inviare `CONNECT_GET_PROTOCOL_INFO` per primo** (MDRP `MDR_packets.md` r.7); le NTFY arrivano solo dopo il GET corrispondente (r.3).
8. **Ritardo di connessione:** GB aspetta 500 ms prima di connettere ("Connecting too fast fails with an IOException", `SonyHeadphonesSupport.java` r.86-90) — osservazione Android, utilità su Windows **NON VERIFICATA**.
9. **Comandi senza risposta** (XM3: INSTRUCTION_GUIDE, upscaling effect, alcuni UPDT): non bloccare.
10. **Alert durante il cambio EQ** (§5.5).
11. **Reboot richiesto** dopo il cambio della funzione del tasto / assistente vocale (GB r.244-250, r.299-309).
12. **Windows:**
    - PLB e MOS usano **Winsock** (`socket(AF_BTH, SOCK_STREAM, BTHPROTO_RFCOMM)` + `SOCKADDR_BTH.serviceClassId` = UUID), non WinRT: PLB `WindowsBluetoothConnector.cpp` r.23-46 e r.133; MOS `ConnectionWindows.cpp` r.78-121 (socket non bloccante, `SO_BTH_AUTHENTICATE` e `SO_BTH_ENCRYPT` attivati, r.94-96).
    - MOS elenca solo i dispositivi **già connessi** (`fReturnConnected = 1`, `ConnectionWindows.cpp` r.255-262); PLB analogo via `BluetoothFindFirstDevice` (r.174). Quindi in pratica la cuffia dev'essere già associata e connessa (A2DP/HFP) a Windows. Che la connessione RFCOMM fallisca se la cuffia non è connessa come audio: **NON VERIFICATO**.
    - Note specifiche su WinRT `RfcommDeviceService` / `StreamSocket`: **nessuna fonte trovata** → **NON VERIFICATO**. Con WinRT l'equivalente sarebbe `RfcommServiceId.FromUuid(<UUID V1>)` + `GetRfcommServicesForIdAsync` (deduzione mia).
    - PLB#29: su Windows 11 con XM4 il client restava su "Sending command…" circa 60 s (gen 2025) — sintomo compatibile con mancata gestione ACK/ritrasmissioni.
13. **Due XM3 connessi contemporaneamente** e XM4 con errore socket "Invalid argument" su macOS sono citati come issue collegate a MOS#56 (#47, #12): non approfonditi.

---

## 7. Licenze: cosa possiamo fare in un progetto MIT / proprietario

Premessa (valutazione generale, non consulenza legale): i **fatti** del protocollo (UUID, valori di byte, layout dei campi, sequenza di handshake, algoritmo di checksum) non sono di per sé proteggibili dal diritto d'autore; lo è l'**espressione** (codice, commenti, testo della documentazione). Per una riscrittura clean-room conviene: (a) estrarre i fatti in questo documento, (b) scrivere il codice Rust senza aprire il codice sorgente delle fonti copyleft, (c) validare contro catture proprie. Le fonti MOS/MDRP/PLB dichiarano di derivare da decompilazione dell'app Sony: questo non cambia la licenza del loro codice, ma è un motivo in più per basarsi sulle **nostre catture** come prova primaria.

| Fonte | Licenza | Uso consentito per noi |
|---|---|---|
| GB (Gadgetbridge) | AGPL-3.0-or-later | **Solo studio.** Nessuna copia di codice, nomi di classi, struttura o commenti. Possiamo usare i fatti (opcode, valori) verificati indipendentemente. |
| AOL (libmdr) | GPL-3.0-or-later | **Solo studio.** Stesse regole di GB. |
| MDRP (mdr-protocol) | nessuna licenza (tutti i diritti riservati) | **Solo studio.** Usare i fatti, non copiare tabelle o testo alla lettera. |
| MOS (mos9527) | MIT | **Riutilizzabile con attribuzione** (mantenere copyright e testo della licenza MIT se si copiano porzioni sostanziali). Per restare clean-room conviene comunque usarlo come riferimento e per le **catture `.bin`** come vettori di test (le catture sono dati; citarne la provenienza). |
| PLB / ANDRO | MIT | **Riutilizzabile con attribuzione**. Attenzione al bug del checksum (§2.5): non copiare `unpackBtMessage`. |
| PLB#29, MOS#56 (issue/PR) | testo di terzi | Solo come informazione. |

---

## Da verificare con le cuffie reali

Catturare (Android HCI snoop log con l'app Sony | Headphones Connect, oppure Wireshark/btsnoop) **per XM3 e per XM4 separatamente**, annotando la versione firmware:

- [ ] Elenco UUID SDP esposti: solo `96CC203E-…-E316F5E069BA` o anche `956C7B26-…`? Canale RFCOMM.
- [ ] Primo frame inviato dall'app (atteso `3e 0c 00 00 00 00 02 00 00 0e 3c`).
- [ ] Risposta `CONNECT_RET_PROTOCOL_INFO` dell'XM4: lunghezza (4 o 8 byte) e versione (XM3 = `0x4010`).
- [ ] Ordine RET/ACK sull'XM4 (la risposta precede l'ACK come sull'XM3?).
- [ ] Comportamento se l'host **non** invia ACK: ritrasmissione? dopo quanto? disconnessione?
- [ ] Il device invia nuove NTFY prima di aver ricevuto l'ACK della precedente?
- [ ] `CONNECT_RET_SUPPORT_FUNCTION` dell'XM4 (elenco funzioni) vs XM3.
- [ ] NC/ASM: byte 3 inviato dall'app (`0x01` o `0x02`), byte 2 (`0x01`, `0x10`, `0x11`) durante e dopo il trascinamento dello slider, range livelli (0..20 o 1..20), focus voice.
- [ ] XM4: esiste la modalità wind reduction (`byte4 = 1`)? Payload NCASM_RET_CAPABILITY `0x61`.
- [ ] Notifica alla pressione del tasto NC/AMB sull'XM4 (atteso `0x69`, preceduto da `0xA9`?).
- [ ] EQ: capability `0x51` dell'XM4 (bande, passi, preset), formato RET `0x57`, SET preset vs bande custom.
- [ ] Spegnimento automatico: l'app usa `0xF8` o `0xFC`? Valori disponibili su XM4.
- [ ] Touch sensor / tasto CUSTOM XM4: tipi GENERAL_SETTING (`0xD1`/`0xD2`/`0xD3`?) e valori.
- [ ] Speak-to-Chat XM4: frame on/off (`F8 05 01 xx`?) e configurazione (`FC 05 00 …`?), notifiche.
- [ ] Batteria: GET `10 00`, RET/NTFY `0x13` con flag ricarica durante la carica USB.
- [ ] Model name / firmware XM4 (`04 01`, `04 02`).
- [ ] Windows: la connessione RFCOMM funziona con Winsock e con WinRT? serve che la cuffia sia già connessa come dispositivo audio? effetto di un'altra app (Sony/altro client) già connessa allo stesso servizio.
- [x] Alert `0x99` (ALERT_NTFY_PARAM) quando si cambia EQ: XM3, 2026-11-08, codec aptX (`19 00 20`). `58 01 16 00` → `99 01 01 01` (fixed message, `DISCONNECT_CAUSED_BY_CONNECTION_MODE_CHANGE`, sì/no). Risposta `98 01 01 00` (no) → `59 01 00 …`, EQ invariato (diverso da quanto riporta MOS). Risposta `98 01 01 01` (sì) → `59 01 00 …`, `E9 01 00 01`, `A5 01 01 00`, codec `1B 00 01` (SBC), canale MDR rimasto aperto; EQ ancora invariato. Poi in SBC `58 01 16 00` → `59 01 16 06 11 0A 0A 0A 0A 0A` e `58 01 FF 06 0D 0A 09 0A 0B 0C` → `59 01 A0 06 0D 0A 09 0A 0B 0C` (bande personalizzate riportate come Manual). XM4: **NON VERIFICATO**. Con il preset già su Manual, nuove bande `58 01 FF 06 …` vengono applicate **senza** `0x59` (solo ACK; la lettura `56 01` dopo le riporta): la notifica arriva solo quando cambia il preset. Per questo l'app tratta i cambi di EQ come richieste senza risposta e rilegge l'EQ. Altre catture dello stesso giorno: (1) bande `58 01 FF …` **uguali** a quelle del preset Sony attivo vengono ignorate (il preset resta, nessuna `0x59`); se diverse, vanno in Manual (`A0`). (2) Il byte del preset nel comando delle bande è ignorato: `58 01 A1 06 …` con Manual attivo ha scritto in Manual. (3) Per scrivere in uno slot si seleziona prima lo slot (`58 01 A1 00`) e poi si mandano le bande: Custom 1 le ha conservate anche dopo il passaggio a Custom 2 e ritorno.
