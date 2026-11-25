# CRU Modern — redesign vizual „Apple-like, ilustrativ, 2027"

Data: 2026-11-25
Stare: propus, așteaptă aprobare

## De ce

Interfața actuală este o temă dark corectă, dar plată: fundal navy uniform, carduri cu rază 14, accent albastru, taburi simple. Utilizatorul vrea un aspect „next-gen Apple-like, ilustrativ, modern 2027": ilustrație de monitor în panoul de detalii, iconițe desenate, mini-vizualizări ale rezoluțiilor și finisaj îngrijit.

## Obiective

- Limbaj vizual coerent „Sequoia restrained" ca bază: spațiu generos, hairline-uri, tipografie fină, accent doar pe acțiuni.
- Un singur moment spectaculos, în panoul de detalii: monitor desenat vectorial, cu rezoluția activă randată în ecran.
- Iconițe și mini-vizualizări desenate cu `egui::Painter` (vectorial, fără asset-uri binare).
- Micro-animații discrete: hover, selecție tab, toast.
- Funcționalitate neschimbată; ambele teme (dark/light) la fel de îngrijite.

## Non-obiective

- Fără funcții noi (EDID, registru, restart rămân exact cum sunt).
- Fără dependențe noi (fără resvg/SVG, fără imagini raster).
- Fără title bar custom; rămâne bara nativă, cu colțuri rotunjite prin DWM.
- Fără blur real de fundal (egui nu suportă); nu simulăm translucență peste conținut.

## Anti-obiective

- Text tehnic mic ilizibil din cauza efectelor.
- Pierdere de spațiu vertical în lista de 10 monitoare.
- Animații care consumă CPU sau fac UI-ul să pară lent.

## Abordări considerate

1. **Glass OS (visionOS)** — translucență și straturi suprapuse. Respinsă ca bază: fără blur real iese „murky" și scade contrastul.
2. **macOS Sequoia restrained** — suprafețe aproape opace, hairline highlight, tipografie fină. Aleasă ca bază.
3. **Bento Showcase 2027** — hero mare + tile-uri. Aleasă doar pentru panoul de detalii, ca să nu aglomereze restul.

Decizie: hibrid 2 + 3 (aleasă de utilizator).

## Design

### Fundal și suprafețe
- Fundal cu gradient vertical discret (dark `#0B0D12 → #0E1117`, light `#F7F8FA → #FFFFFF`).
- Carduri: rază 18, umplere opacă (`#12151C` dark / `#FFFFFF` light), border 1px la 6% contrast, plus un hairline luminos sus (gradient) pentru senzația de sticlă.
- Umbră în două straturi (apropiată, densă + difuză), nu una singură.
- Zona de conținut are padding 20–24; cardurile respiră.

### Tipografie
- `SegUIVar.ttf` (Segoe UI Variable) dacă există, fallback `segoeui.ttf`; semibold pentru titluri.
- Hero 22–24, titlu card 15, body 13, meta 11. Tracking ușor strâns la titluri mari.
- Numerotarea rezoluțiilor în tabular/monospace doar pentru cifre, ca să nu danseze coloanele.

### Culori
- Accent: `#0A84FF` (dark) / `#007AFF` (light), buton principal cu gradient vertical subtil.
- Danger: `#FF453A` / `#FF3B30`.
- Succes/avertizare: verde/amber Apple, folosite doar în chips-uri.

### Controale
- Butoane-pilulă (rază = înălțime/2), înălțime 30–32.
- Taburi = segmented control: track adâncit, pilulă selectată cu umbră mică; tranziție cu `animate_bool`.
- Iconițe circulare 28–30, hover cu fundal 8%.

### Iconițe desenate (`ui/icons.rs`)
Funcții `painter`-based, linie 1.5px, rotunjită: logo (ecran + undă), monitor, refresh, import, export, soare/lună, plus, minus, săgeți, scut (admin), check, alertă.

### Ilustrație monitor (`ui/illustration.rs`)
- Monitor stilizat: ecran cu rază 10, bezel subțire, stand trapezoidal, glow foarte subtil sub ecran.
- În ecran: rezoluția activă (`1920 × 1080`), refresh-ul și un dreptunghi de aspect scalat.
- Dimensiune ~240×160, se scalează cu DPI; culori din paletă, deci arată bine în ambele teme.
- Opțional: un al doilea monitor mic în spate, decalat, când există mai multe display-uri active.

### Mini-vizualizare rezoluții
- În tabel, înaintea textului: dreptunghi de aspect (ex. 16:9) desenat la 26×16, colț 3, umplere accent-soft, contur hairline.
- Nu înlocuiește coloanele; doar adaugă un reper vizual.

### Mișcare
- `ctx.animate_bool` pentru hover/selecție (120–160 ms).
- Toast: fade + rise 8px, dispare după 4 s, cu iconiță desenată.
- Fără animații pe date/tabele.

### Fereastră
- Colțuri rotunjite Win11 via `DwmSetWindowAttribute(DWMWA_WINDOW_CORNER_PREFERENCE=33, DWMWCP_ROUND)`, păstrat dark title bar.

## Fișiere atinse

- `src/ui/theme.rs` — paletă, rază, umbre, fonturi, componente (buton, segmented, icon, chip, toast).
- `src/ui/icons.rs` — nou: iconițe vectoriale.
- `src/ui/illustration.rs` — nou: monitor + mini aspect.
- `src/ui/app.rs` — hero în detalii, sidebar, taburi segmentate, toast animat, status bar.
- `src/ui/win.rs` — colțuri rotunjite.
- `src/ui/mod.rs` — module noi.

## Ce rămâne la latitudinea implementării

Valori exacte de culoare, forme de iconițe, durate de animație, spacing fin.
