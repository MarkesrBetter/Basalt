# Basalt Design Read

## Mimari Ayrım
- **Masaüstü İstemci Kabuğu (`basalt.exe` / `src-tauri` + `frontend/`):** Tauri 2 ve Windows WebView2 üzerinde çalışan anlamsal HTML, CSS ve TypeScript WebUI. Kalıcı ayarlar, kayıt klasörü yönetimi, donanım analizi, yerel skin doğrulama, TCP SLP ping ve Gradle süreç takibi Rust çekirdeğinde (`basalt` + `src-tauri`) çalışır.
- **Gerçek Minecraft 1.21.4 İstemcisi (`client-gradle/`):** Masaüstü arayüzü `gradlew.bat runClient` ile gerçek Minecraft Java Edition `1.21.4` (Fabric Loader `0.20.1`, Loom `1.15.5`, Fabric API `0.119.4+1.21.4`) sürecini başlatır; `options.txt` eşitlemesi ve `BASALT_*` ortam değişkenleri üzerinden kullanıcı adı, UUID, RAM (`-Xmx`), GC (`G1GC` / `ZGC`), yerel PNG skin ve Quick Play parametreleri aktarılır.

## Kadran Beyanı
`ENERGY 1 / RHYTHM 2 / MOTION 1`

Obsidyen Mor ana kimlik üzerine kurulu, donanım odaklı 10 mineral/biyom teması (**Obsidyen Mor**, **Kömür Bakır**, **Açık Kireçtaşı**, **Ladin ve Zümrüt**, **Derin Lapis**, **Kızıl Bazalt**, **Açık Buzul**, **Antik Şehir**, **End Taşı**, **Kiraz Çiçeği**), bağımsız vurgu rengi / özel `#RRGGBB` aksan kalibrasyonu, 3 köşe profili (`Keskin`, `Dengeli`, `Yumuşak`), 5 pencere içi masaüstü düzeni (**Sol Ray**, **Üst Şerit**, **Odak Kompakt**, **Sağ Ray**, **İnce Dock**), tek üst çubuklu gerçek OS tam ekranlı **Tam Ekran Konsol Modu (`DeckMode`, `#console-home`)**, ince özel kaydırma çubukları ve gerçek Minecraft `1.21.4` `.ogg` ses efektleri (`src/sounds.rs` + `sounds.ts`).

## Renk Sistemi ve WCAG 2.2 AA Tokenları (10 Tema)
- **Obsidyen Mor (`dark`, varsayılan, `frontend/src/styles/tokens.css`):** `#131118` obsidyen zemin, `#201C29` yüzey, `#A47CFF` / `#B593FF` ana Basalt mor aksanı (`#0E091B` buton metni).
- **Kömür Bakır (`ember`):** `#121417` bazalt zemin, `#1D2127` yüzey, `#E0824A` / `#EB9560` bakır aksanı (`#140B06` buton metni).
- **Açık Kireçtaşı (`light`):** `#F3F1EC` zemin, `#FFFFFF` yüzey, `#B34C19` yanık bakır aksanı (`#FFFFFF` buton metni).
- **Ladin ve Zümrüt (`spruce`):** `#101613` orman koyusu zemin, `#1A241F` yüzey, `#56C27E` zümrüt aksanı (`#08170E` buton metni).
- **Derin Lapis (`cobalt`):** `#0F141C` gece kayaç zemin, `#1A2230` yüzey, `#53B8EC` buz camgöbeği aksanı (`#06141D` buton metni).
- **Kızıl Bazalt (`nether`):** `#161113` Nether koyusu zemin, `#241B1F` yüzey, `#EA6B66` kızıltaş aksanı (`#1A0707` buton metni).
- **Açık Buzul (`frost`):** `#EDF2F7` serin mineral açık zemin, `#FFFFFF` yüzey, `#1D5FB8` kobalt aksanı (`#FFFFFF` buton metni).
- **Antik Şehir (`sculk`):** `#0A1013` derin sculk zemin, `#131E25` yüzey, `#35CAD0` biyolüminesan camgöbeği aksanı (`#03181A` buton metni).
- **End Taşı (`end`):** `#14130F` void koyusu zemin, `#211F18` yüzey, `#E4BE52` saf altın aksanı (`#1A1302` buton metni).
- **Kiraz Çiçeği (`cherry`):** `#F7EEF2` açık pastel koruluk zemini, `#FFF9FB` yüzey, `#B52A5C` yakut aksanı (`#FFFFFF` buton metni).
- **Bağımsız Vurgu Rengi (`colors.ts`):** `Auto`, `Copper`, `Emerald`, `Cyan`, `Amber`, `Crimson`, `Amethyst` ön ayarları ve canlı WCAG kontrast kalibrasyonlu özel `#RRGGBB` renk seçici.

## Yerleşim Görünüşleri (6 Mod)
- **Sol Ray (`Sidebar`):** Sabit sol dikey menü (`214px`) ve iki sütunlu başlatma tezgahı.
- **Üst Şerit (`Topbar`):** Yatay tam genişlikte gezinme başlığı ve geniş sahne.
- **Odak Kompakt (`CompactDeck`):** Ortalanmış sıkı enstrüman çerçevesi (`max-width: 980px`).
- **Sağ Ray (`RightRail`):** Sağ kenara yerleştirilmiş menü ve başlatma aksiyonunun öne alındığı ters tezgah düzeni.
- **İnce Dock (`MinimalRail`):** Simge odaklı dar (`88px`) dikey gezinme çubuğu.
- **Deck Konsol / Tam Ekran Konsol Modu (`DeckMode`):** Uygulama açılışta seçili masaüstü düzeninde (varsayılan `Sidebar`, sesler kapalı) başlar. Üst çubuktaki **Konsol Modu (Tam Ekran)** düğmesi (`#btn-enter-deck`) veya profil seçimi üzerinden `set_fullscreen(true)` ile gerçek tam ekrana geçilir; PlayStation / Steam Deck Big Picture düzeninde dinamik SVG kahraman vitrini, oyuncu skin kaidesi, yatay dünya/sunucu/menü kart karuseli (`#deck-carousel`), dijital saat (`#deck-clock`), Gamepad (`A`/`B`/`Y`/`L1`/`R1` + analog/D-Pad) ile klavye (`Ok tuşları`/`Q`/`E`/`M`/`Esc`) kısayol şeridi (`#deck-controls-bar`) etkinleşir ve konsol sesleri otomatik açılır. `Esc`, `B` veya **Masaüstüne Dön** masaüstü penceresine (`set_fullscreen(false)`) geri döner.

## Ses, Pencere Kromu ve Orantılı Ölçekleme
- `src/sounds.rs` + `frontend/src/ts/sounds.ts`: Telifli ses dosyalarını `basalt.exe` içine gömmeden Minecraft `1.21.4` gerçek `.ogg` ses varlıklarını (`random/click.ogg`, `random/click_stereo.ogg`, `ui/toast/in.ogg`, `ui/toast/out.ogg`, `random/orb.ogg`, `random/wood_click.ogg`, `random/levelup.ogg`, `note/bass.ogg`) önce yerel Fabric Loom veya `.minecraft/assets` önbelleğinden, yoksa resmi `resources.download.minecraft.net` uç noktasından `%LOCALAPPDATA%\Basalt\Basalt\data\sounds\` dizinine önbellekler. Web Audio `AudioContext.decodeAudioData` + `HTMLAudioElement` `blob:` havuzu üzerinden oynatır. Normal masaüstünde varsayılan olarak sessizdir (`ui_sounds = false`); Tam Ekran Konsol Modunda sesler otomatik açılır.
- **Özel Çerçevesiz Pencere Kromu (`decorations: false` + `.app-titlebar`):** Yerel Windows başlık çubuğu yerine tüm tema ve düzenlerle bütünleşen sürüklenebilir Basalt başlık çubuğu (`window_start_drag`, `window_minimize`, `window_toggle_maximize`, `window_is_maximized`, `window_close` Tauri IPC komutları ve özel SVG pencere simgeleri) kullanılır.
- **Orantılı Viewport Ölçekleme ve Sıfır Varsayılan Kaydırma Çubuğu (`base.css`, `layout.css`, `components.css`):** Kök yazı tipi boyutu viewport ile orantılı büyür (`clamp(11.2px, min(1.06vw, 1.7vh), 22px)`), sahne genişlik sınırları kaldırılır ve tüm sekmeler (Profil iki sütunlu yerleşim, Donanım/Grafik ızgaraları, 6 kartlı konsol şeridi) standart çözünürlüklerde tek ekrana sığarak kaydırma çubuğu (`scrollbar-width: none`) göstermez.
- Masaüstü Oyna ekranı kutu yığınları yerine birleşik mimari kahraman sahnesi (`.play-hero`) ile ince çizgi ayrımlı alt veri ızgarası (`.play-under-grid`) kullanır; Konsol Modu gereksiz ikinci üst çubuğu gizler, saat ile oyuncu rozetini tek üst şeritte sunar ve kapak sanatı içeren yatay karusel (`.deck-card-cover`) ile tam viewport yüksekliğini dengeler.

## Tipografi ve Bilinçli Dışlamalar
- `Segoe UI Variable` / `Segoe UI` sistem yazı tipi ve sayısal telemetri için `Consolas` (`tabular-nums`).
- Kutu kafesleri, gereksiz lineer geçiş yıkamaları, dekoratif emojiler, sahte KPI kutuları ve em dash karakteri kullanılmaz.
