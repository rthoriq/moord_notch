# Catatan perubahan

Riwayat pengembangan NotchWindows, tahap demi tahap. Cara pasang dan pakai yang terbaru ada di
[README](README.md).

## Tahap 1: dasar notch

- Pil hitam di tengah atas layar, menyatu dengan tepi layar seperti notch MacBook.
- Arahkan kursor ke pil (atau klik) → mengembang jadi panel. Kursor keluar → menutup lagi.
  Animasinya memakai pegas sehingga sedikit memantul.
- Maskot sepasang mata yang sesekali berkedip.
- Mengecil jadi garis tipis saat jendela aktif di-maximize, supaya tidak menutupi tab browser.
- Sembunyi otomatis saat ada aplikasi full-screen (game, video, presentasi).
- Ikon tray: tampilkan/sembunyikan notch, "Mulai bersama Windows", dan keluar.
- Klik di luar bentuk notch tembus ke jendela di bawahnya.

## Tahap 2: musik dan kabar singkat

- **Musik/video**: membaca apa pun yang muncul di kontrol media Windows (Spotify, YouTube di
  browser, Media Player, dll). Saat diputar, pil menampilkan sampul kecil dan gelombang yang
  diwarnai sesuai sampul. Panel menampilkan sampul, judul, artis, progres, serta tombol
  sebelumnya / putar-jeda / berikutnya.
- **Volume**: HUD berisi ikon, bilah, dan persen setiap kali volume atau mute berubah.
  HUD volume bawaan Windows 11 tetap muncul, karena tidak bisa dimatikan secara resmi.
- **Daya**: kabar saat charger dicolok atau dicabut, dan peringatan saat baterai turun ke 20% dan 10%.
- **Bluetooth**: kabar saat perangkat yang sudah dipasangkan tersambung atau terputus, termasuk
  persen baterainya bila perangkat melaporkannya.

## Tahap 3: widget di panel

Panel punya lima halaman: **Beranda** (jam, atau pemutar musik kalau ada media), **Shelf**,
**Pomodoro**, **Minum air**, dan **Tugas**. Pindah halaman dengan roda mouse / geser dua jari,
panah di kiri-kanan panel, atau klik titik di bawah panel. Halaman terakhir diingat.

- **Shelf**: seret file atau folder ke notch, maka panel terbuka di halaman Shelf dan file
  tersimpan di sana. Yang disimpan hanya lokasinya, bukan salinan file. Klik dua kali untuk
  membuka, klik kanan untuk menampilkannya di File Explorer, seret keluar untuk menyalin atau
  memindahkannya ke aplikasi lain. File yang dipindahkan otomatis hilang dari shelf.
- **Pomodoro**: fokus 25 menit lalu istirahat 5 menit yang langsung berjalan. Klik cincin untuk
  mulai/jeda. Selama berjalan, pil menampilkan cincin progres dan sisa menit. Saat waktu habis
  muncul kabar singkat disertai bunyi notifikasi Windows.
- **Minum air**: target 2 L per hari, satu klik "+ Minum" = 250 ml, ada grafik tujuh hari terakhir.
  Pengingat muncul satu jam setelah minum terakhir, lalu tiap 30 menit selama belum minum,
  hanya pukul 07.00–22.00 dan selama target belum tercapai.
- **Tugas**: klik kotak "Tambah tugas" lalu ketik. Enter menyimpan dan siap untuk tugas berikutnya,
  Esc membatalkan, Ctrl+V menempel. Klik tugas untuk mencentang. Daftar panjang digulir dengan
  roda mouse.

Mengklik kabar singkat (misal pengingat minum) membuka halaman yang bersangkutan.

Data disimpan sebagai file teks biasa di `%APPDATA%\NotchWindows`: `shelf.txt`, `todo.txt`,
`water.txt`, dan `settings.txt`. Semuanya bisa dibaca dan diedit dengan Notepad.

## Tahap 4: privasi, lirik, sistem, dan tema

- **Indikator kamera/mikrofon**: titik hijau (kamera) atau oranye (mikrofon) di ujung kanan pil
  selama ada aplikasi yang memakainya, plus kabar singkat "Kamera sedang dipakai · Zoom" saat
  aplikasi baru mulai memakainya. Mikrofon dibaca dari sesi rekam Windows (Core Audio), kamera
  dari pemantau sensor Media Foundation; keduanya mengabari notch tanpa polling. Selama indikator
  menyala, notch tidak menciut jadi garis tipis supaya titiknya tetap terlihat.
- **Lirik (opsional)**: nyalakan lewat menu tray "Lirik lagu (memakai internet)". Lirik
  tersinkron diambil dari [LRCLIB](https://lrclib.net) lewat HTTPS. Yang dikirim hanya judul,
  artis, album, dan durasi lagu yang sedang diputar; hasilnya hanya disimpan di memori. Selama
  lagu diputar, pil menampilkan baris yang sedang dinyanyikan dan lebarnya menyesuaikan panjang
  baris; baris yang terlalu panjang dibagi jadi dua baris. Di panel, lirik tampil di kolom lirik
  (lihat Tahap 9).
- **Sistem**: halaman keenam berisi beban CPU, GPU, RAM, dan disk C: (warnanya berubah oranye di
  atas 60% dan merah di atas 85%), kecepatan jaringan, lama laptop menyala, dan aplikasi yang
  sedang memakai kamera/mikrofon. CPU dan GPU memakai angka yang sama dengan Task Manager.
  Hanya diukur selama halaman ini terbuka.
- **Tema**: pilih di menu tray → Tema: Hitam (bawaan), Grafit, Biru malam, Anggur, Kaca, atau Terang.
  Kaca membuat notch setengah tembus pandang dengan garis tepi tipis (tanpa blur, karena blur
  di Windows hanya bisa lewat API yang tidak resmi).

## Perbaikan setelah Tahap 4

Semua pengaturan ada di menu tray → **Pengaturan**.

- **Suara notifikasi**: bunyi bawaan Windows untuk pengingat minum, pomodoro dan timer selesai,
  charger, Bluetooth, Wi-Fi, unduhan, baterai lemah, dan kamera/mikrofon. Diam otomatis saat
  full-screen, presentasi, atau Focus/Jangan Ganggu. Volume, kecerahan, Caps Lock, dan pesan masuk
  sengaja tanpa bunyi dari notch (aplikasi pengirim pesan sudah berbunyi sendiri).
- **Lock screen**: notch tetap tampil saat laptop dikunci (Win+L), tapi hanya jam dan kontrol
  musik; halaman lain tidak bisa dibuka, dan kabar pesan hanya berbunyi "Pesan baru" tanpa nama
  pengirim. Layar isi PIN/kata sandi adalah desktop aman Windows, jadi di sana tidak ada aplikasi
  yang bisa tampil.
- **Caps Lock / Num Lock**: kabar singkat "Nyala"/"Mati". Notch hanya memperhatikan dua tombol
  ini; tombol lain langsung diabaikan dan tidak pernah disimpan.
- **Kecerahan layar**: kabar berisi bilah dan persen setiap kecerahan berubah.
- **Pesan masuk** (WhatsApp, atau semua aplikasi chat): kabar berisi logo aplikasi, pengirim,
  dan potongan pesan. Klik kabarnya untuk membuka aplikasinya. "Sembunyikan isi pesan" hanya
  menampilkan pengirim. Windows hanya mengizinkan aplikasi berpaket membaca notifikasi, jadi
  fitur ini perlu diaktifkan sekali lewat menu tray → Pengaturan → Pesan masuk → Aktifkan pesan
  masuk (lihat README). Setelah itu pilih jenisnya di menu yang sama.

Kalau izin pernah ditolak, nyalakan lagi di Pengaturan Windows → Privasi & keamanan →
Notifikasi → izinkan aplikasi mengakses notifikasi.

**WhatsApp Web** dan situs chat lain di browser: Windows hanya memberi judul dan isi
notifikasi browser, tanpa alamat situs pengirimnya, jadi notch tidak bisa membedakan WhatsApp Web
dari situs lain. Nyalakan **Pesan masuk → Notifikasi browser (WhatsApp Web, dll.)** untuk
menampilkan semua notifikasi browser sebagai pesan (berlabel nama browser, misal "Brave"). Supaya
isinya hanya pesan chat, izinkan notifikasi hanya untuk situs chat di browser (Brave:
`brave://settings/content/notifications`). Mengklik kabarnya membawa ke jendela WhatsApp kalau
ada, lalu ke jendela browsernya. Untuk logo dan nama yang tepat, pakai aplikasi WhatsApp Desktop.

Untuk memeriksa apa yang dilihat notch, jalankan (bisa tanpa Administrator):

```bash
"%LOCALAPPDATA%\Programs\NotchWindows\notch.exe" --cek-notifikasi
```

Hasilnya ada di `%TEMP%\notch-cek-notifikasi.txt`: status izin dan daftar notifikasi di pusat
notifikasi beserta cara notch memilahnya. Isi pesan dan nama pengirim tidak ikut ditulis.

## Tahap 5: pintasan, halaman, maskot, dan kabar baru

- **Pintasan Win+Alt+N**: membuka panel, tekan lagi untuk menutup. Kalau kursor tidak datang ke
  notch, panel menutup sendiri setelah 5 detik. Windows hanya mengabari saat kombinasi ini
  ditekan; tombol lain tidak dibaca. Bisa dimatikan di Pengaturan.
- **Atur halaman**: menu tray → Halaman untuk menyembunyikan/menampilkan halaman (Beranda selalu
  ada). Klik kanan di panel untuk menggeser halaman yang sedang dibuka ke kiri/kanan,
  menyembunyikannya, atau menampilkan lagi halaman yang tersembunyi. Halaman yang disembunyikan
  ikut mematikan fiturnya: Minum air → tanpa pengingat, Shelf → tidak menerima file dan screenshot.
- **Ukuran notch**: menu tray → Ukuran: Kecil, Sedang (bawaan), atau Besar.
- **Maskot lebih hidup**: mata melirik ke arah kursor dan kembali menatap ke depan kalau kursor
  lama diam; setengah terpejam pukul 23.00–05.00; tersenyum "^ ^" saat target minum tercapai atau
  sesi fokus selesai; membulat kaget saat baterai lemah atau kamera/mikrofon mulai dipakai.
- **Screenshot masuk shelf**: hasil Win+Shift+S, PrtScn, atau Win+PrtScn yang tersimpan di
  `Pictures\Screenshots` langsung masuk shelf dengan thumbnail-nya, siap diseret ke WA/Discord.
- **Unduhan selesai**: kabar saat file baru di folder Downloads selesai diunduh (file sementara
  browser seperti `.crdownload`/`.part` diabaikan sampai selesai). Klik kabarnya untuk melihat
  file di File Explorer; file tidak pernah dibuka otomatis.
- **Wi-Fi**: kabar saat tersambung (beserta nama jaringannya), terputus, atau kehilangan akses
  internet. Nama jaringan diambil dari nama profilnya, jadi tidak perlu izin lokasi.
- **Suhu GPU** di halaman Sistem, dari data driver grafis yang sama dengan Task Manager
  (oranye mulai 78 °C, merah mulai 87 °C). Suhu CPU tidak ditampilkan karena Windows tidak
  menyediakannya tanpa hak admin atau driver tambahan.

## Tahap 6: kontrol cepat, timer, cuaca, dan mode rapat

- **Kontrol cepat** (halaman baru): slider volume dan kecerahan (klik, seret, atau putar roda
  mouse di atasnya), klik ikon speaker untuk bisu, tombol bisukan mikrofon, dan **Senyap 1 jam**:
  notch tidak memunculkan kabar dan tidak berbunyi, kecuali baterai lemah dan timer habis.
  Windows tidak menyediakan cara resmi untuk menyalakan Jangan Ganggu dari aplikasi lain, jadi
  senyap ini hanya untuk notch. Kecerahan hanya untuk layar laptop; monitor eksternal diatur dari
  tombolnya sendiri.
- **Timer cepat** (halaman baru): 1, 5, 10, atau 15 menit, bisa dijeda dan ditambah semenit.
  Selama berjalan, pil menampilkan cincin oranye dan sisa waktunya (detik di menit terakhir).
  Saat habis muncul kabar "Timer selesai" dengan bunyi alarm Windows.
- **Cuaca (opsional, memakai internet)**: menu tray → Pengaturan → Cuaca. Saat pertama dinyalakan,
  panel meminta nama kota. Data dari [Open-Meteo](https://open-meteo.com), tanpa akun dan tanpa
  API key. Yang dikirim hanya nama kota yang diketik (sekali, untuk mencari koordinatnya) lalu
  koordinat pusat kota itu tiap 30 menit; lokasi laptop tidak pernah dibaca. Cuaca tampil di
  halaman jam ("29° Cerah · Jakarta") dan di tombol kota pada halaman kontrol. Ganti kota dengan
  mengklik tombol itu.
- **Mode rapat** (menyala secara default): selama kamera atau mikrofon dipakai (Zoom, Meet, saat
  berbagi layar), kabar pesan hanya berbunyi "Pesan baru" tanpa pengirim dan isinya, dan notch
  tidak berbunyi (bisa terdengar peserta lain), kecuali kabar bahwa kamera/mikrofon baru dinyalakan.

## Tahap 7: tampilan ala notch Mac

Mengikuti aplikasi notch populer di Mac (Boring Notch, NotchDrop, NotchNook):

- **Seret file**: begitu file masuk ke notch, panel langsung terbuka memantul ke halaman Shelf
  dengan area lepas bergaris putus-putus yang berjalan. Saat file tepat di atasnya, area itu
  menyala warna aksen dan ikonnya mengangguk. Sesudah dilepas, area lepas berpendar sekali dan
  ubin baru muncul membesar; ubin yang dihapus mengecil lalu hilang, ubin yang disorot membesar 5%.
- **Buka/tutup**: membuka sedikit memantul, menutup mulus tanpa pantulan. Isi panel muncul
  membesar dari 80% dengan titik tumpu di atas. Sudut notch memakai kurva yang lebih lembut dan
  ikut membesar saat terbuka.
- **Bayangan** lembut di bawah notch saat terbuka, dan tipis saat disentuh kursor.
- **Intip saat disentuh**: pil langsung sedikit membesar saat disentuh kursor, baru terbuka
  penuh sesaat kemudian. Garis tipis (saat jendela di-maximize) juga kembali jadi pil dulu.
- **Kabar ringkas ala Dynamic Island** (menyala secara default, menu tray → Pengaturan → Kabar
  ringkas): volume, kecerahan, Caps/Num Lock, dan charger tampil setinggi pil dengan notch melebar
  ke samping. Angka bergulir ke nilai baru dan bilahnya bergeser halus. Kabar lain (pesan,
  Bluetooth, unduhan, dll.) tetap berbentuk kartu karena isinya dua baris.

## Tahap 8: kalender, gestur, unduhan, dan bagikan

- **Kalender di Beranda** (menu tray → Pengaturan → Kalender di Beranda, menyala secara default):
  Beranda melebar dengan musik atau jam di kiri dan kalender di kanan. Deretan lima hari bergeser
  (klik hari, atau putar roda mouse di atasnya), hari ini ditandai biru, dan hari yang punya agenda
  diberi titik. Kepala kolom menampilkan cuaca (kalau aktif) dan persen baterai.
- **Agenda sendiri**: klik "+ Tambah agenda" lalu ketik, misal `09.00 Rapat tim` atau
  `Beli kado 15:30` (jam di awal atau akhir; tanpa jam = sepanjang hari). Enter menyimpan ke hari
  yang sedang dipilih. Klik lingkaran untuk menandai selesai, sorot lalu klik × untuk menghapus.
  Saat jamnya tiba muncul kabar "Agenda sekarang" dengan bunyi pengingat. Disimpan di
  `%APPDATA%\NotchWindows\agenda.txt`; agenda yang lewat lebih dari 60 hari dibuang. Di lock screen
  kalender tidak ditampilkan karena agenda itu pribadi.
- **Gestur touchpad**: geser dua jari ke kanan/kiri di pil selama musik diputar untuk lagu
  berikutnya/sebelumnya.
- **Unduhan yang berjalan**: selama browser mengunduh ke folder Downloads, pil melebar dan
  menampilkan ikon unduh serta ukuran yang terus bertambah (persennya tidak diketahui karena browser
  tidak memberi tahu ukuran totalnya). Musik dan timer tetap didahulukan di pil.
- **Zona Bagikan**: saat file diseret ke notch, area lepas terbagi dua: "Simpan di shelf" dan
  **"Bagikan"**. Melepas file di Bagikan membuka jendela Bagikan Windows (Berbagi di Sekitar ke PC
  lain, ponsel lewat Phone Link, email, WhatsApp, dll.), padanan AirDrop di Mac. Notch sendiri tidak
  mengirim apa pun; tujuannya dipilih di jendela Windows itu.
- **Maskot bangun**: saat notch muncul, setelah buka kunci, dan setelah laptop bangun dari tidur,
  mata maskot terpejam sebentar lalu terbuka perlahan dan berkedip.

## Tahap 9: tampilan musik baru

- **Pemutar baru**: sampul, judul tebal, dan artis di atas; waktu berjalan, bar, dan sisa waktu di
  tengah; tombol lirik (kiri) serta ⏮ ⏯ ⏭ (tengah) di bawah. Kepala pemutar berisi tanggal dan jam
  di tengah dengan cincin baterai (hijau saat mengisi, merah saat hampir habis), cuaca di kiri
  (kalau aktif), dan gelombang musik di kanan.
- **Kolom lirik**: tombol gelembung kata di bawah waktu berjalan menampilkan/menyembunyikan lirik
  di samping pemutar. Dengan kalender, kalender bergeser keluar lalu lirik bergeser masuk; tanpa
  kalender, notch melebar ke samping. Tiga baris terlihat: baris sebelumnya redup, baris yang
  sedang dinyanyikan putih tebal (sampai dua baris), baris berikutnya redup; saat baris berganti
  semuanya bergulir naik. Tombolnya hanya muncul kalau "Lirik lagu (memakai internet)" di menu
  tray sudah dinyalakan, dan pilihannya diingat.
- **Lirik untuk YouTube dan YouTube Music** (di browser): judul video seperti
  "Mahalini - Sial (Official Music Video)" dari channel "MahaliniVEVO" dirapikan dulu jadi judul
  "Sial" dan artis "Mahalini" (tambahan seperti "(Official Lyric Video)", "[MV]", "| Lirik",
  "ft. …", serta channel "… - Topic" dibuang). Pencarian bertahap: judul asli, judul yang dirapikan,
  lalu pencarian bebas; dari semua hasil dipilih versi yang durasinya paling dekat dengan lagu yang
  diputar, supaya lirik tidak telat atau kecepatan. Lirik dari YouTube sendiri tidak dibaca (tidak
  ada jalur resmi untuk itu).
- **Seret bar untuk lompat**: bar progres menebal saat disorot; klik atau seret untuk lompat ke
  bagian lagu lain. Hanya untuk aplikasi yang mengizinkannya di kontrol media Windows (misal Spotify
  dan YouTube di browser); kalau tidak, bar tidak bisa diklik.
- **Pendaran warna**: bagian bawah panel berpendar warna sampul lagu, oranye selama timer berjalan,
  dan merah/hijau selama pomodoro berjalan.
- **Halaman lain disamakan**: tombol putar/jeda berbentuk padat seperti di pemutar, slider volume
  dan kecerahan tanpa kenop dan menebal saat disorot seperti bar lagu, judul halaman lebih
  terang, dan halaman jam memakai kepala yang sama (cuaca di kiri, baterai di kanan).

## Tahap 10: siap dirilis di GitHub

- **Pemasang tanpa Administrator**: notch.exe yang dibuka dari luar folder pemasangan (misal dari
  folder Unduhan) menawarkan untuk memasang diri ke `%LOCALAPPDATA%\Programs\NotchWindows`, atau
  memperbarui versi yang terpasang. Notch muncul di menu Start, di Pengaturan > Aplikasi (bisa
  dicabut dari sana), dan berjalan bersama Windows. Tanpa dialog: `notch.exe --pasang` dan
  `notch.exe --cabut --diam`.
- **Aktifkan pesan masuk dari menu tray**: paket identitas dibuat, ditandatangani, dan didaftarkan
  oleh notch sendiri (tanpa Windows SDK). Kunci tanda tangannya dihapus begitu paket selesai
  ditandatangani. Menggantikan `packaging\install.ps1`.
- **Sambutan pertama kali**: panel singkat cara memakai notch, sampai tombol "Mengerti" diklik.
- **Log error lokal** di `%APPDATA%\NotchWindows\log.txt` (menu tray → Pengaturan → Buka folder
  log), tanpa isi pesan, judul lagu, atau nama file.
- **Ikon dan info versi** di notch.exe ("NotchWindows" di Task Manager dan properti file).
- **Build dan rilis otomatis** di GitHub Actions, lengkap dengan checksum dan bukti asal build.
- Lisensi MIT.

## Perbaikan setelah Tahap 10

- Saat jendela aktif di-maximize, notch tetap menciut jadi garis tipis walau kamera atau mikrofon
  sedang dipakai. Titik privasinya (hijau = kamera, oranye = mikrofon) tetap tampil di ujung kanan
  garis, jadi ruang tab tetap lega tanpa kehilangan tanda privasi.
- Mata tersenyum "^ ^" sedikit lebih renggang, jadi kedua lengkungnya tidak lagi menyatu seperti
  huruf "m" (terutama terlihat di halaman jam).
- Paket aset video: `cargo test aset_video -- --ignored` merender semua tampilan notch sebagai PNG
  transparan 4× dan animasinya sebagai urutan frame 60 fps ke folder `aset-video`.
- Video promosi Story Instagram: `cargo test --release video_promosi -- --ignored` merender
  `aset-video/NotchWindows-story.mp4` (1080×1920, 30 fps, 28 detik) langsung dari renderer
  notch; `video_cuplikan` menyimpan satu frame per adegan untuk dicek dulu. Musik dan efek
  suaranya disintesis dari kode (120 BPM, bebas hak cipta, -14 LUFS) dan jatuh tepat di potongan
  adegan; `video_musik` merender musiknya saja sebagai WAV beserta gambar spektrogramnya.
- Video Reels lengkap (±3 menit): `cargo test --release reels_video -- --ignored` merender
  `aset-video/NotchWindows-reels.mp4` berisi semua fitur dalam 10 topik, masing-masing dengan
  animasinya sendiri dan nama topik singkat sebagai transisi. Per topik: `BAB=3 cargo test
  --release reels_bab -- --ignored`; cuplikan frame: `reels_cuplikan`.
- Musik Reels dari lagu bebas royalti di `aset-video/lagu.mp3` (tidak ikut git): tempo, awal
  birama, dan kuncinya dibaca otomatis (`reels_lagu` menggambar hasilnya), setiap adegan
  dibulatkan ke ketukan lagu, pergantian topik jatuh di garis birama, efek suara bernada digeser
  ke kunci lagu, dan lagu yang lebih pendek dari video diperpanjang dengan mengulang satu frasa
  8 birama. Tanpa file itu, musik sintesis yang dipakai. Lagu yang dipakai: "Julia" oleh
  grand_project (Pixabay).

## Tahap 11: jendela Pengaturan

Semua pengaturan pindah dari menu tray ke jendela **Pengaturan** bergaya Windows 11: latar Mica,
terang atau gelap mengikuti Windows, warna aksen Windows, dan tajam di semua skala layar.

- **Kategori di kiri**: Umum (tampilkan notch, mulai bersama Windows, suara, keluar), Tampilan
  (enam tema dengan contoh notch, tiga ukuran, kalender di Beranda, kabar ringkas), Halaman
  (tampilkan/sembunyikan dan urutan ↑↓), Kabar, Pesan masuk (status izin dan tombol Aktifkan),
  Privasi & internet (mode rapat, lirik, cuaca, ganti kota), Pintasan, dan Tentang (versi, GitHub,
  folder data dan log).
- **Cara membuka**: klik ikon tray; buka NotchWindows lagi dari menu Start saat notch sudah
  berjalan; atau klik "Pengaturan" di kepala halaman Kontrol cepat.
- **Langsung berlaku** tanpa tombol Simpan. Perubahan dari tempat lain (klik kanan panel, kota
  cuaca, izin pesan) langsung terlihat di jendela yang sedang terbuka.
- **Keyboard**: Tab/Shift+Tab berpindah kontrol, Spasi/Enter memilih, panah atas/bawah berpindah
  kategori, Page Up/Down menggulir, Esc menutup.
- **Menu tray** tinggal "Pengaturan…", "Tampilkan notch", dan "Keluar" (klik kanan).
- Keterangan yang panjang turun ke baris berikutnya, dan daftar kategori menyempit saat jendelanya
  sempit.
- `cargo test settings_window_previews` menggambar setiap kategori (terang, gelap, dan jendela
  tersempit) ke `target/preview/pengaturan-*.png`.
- **Ikon taskbar**: jendela Pengaturan sempat tampil tanpa ikon di taskbar saat pesan masuk aktif.
  Windows mengambil ikonnya dari logo paket identitas di folder pemasangan, padahal logo itu tidak
  ada di sana dan warnanya hitam. Sekarang notch melengkapi logo itu sendiri saat dijalankan, logo
  paket dibuat dari maskot yang sama dengan ikon notch.exe, dan jendela Pengaturan memakai ikon
  notch.exe di taskbar (tanpa pelat warna aksen di sudutnya).

## Tahap 12: panggilan aktif

Saat aplikasi apa pun memakai mikrofon lebih dari 3 detik (Zoom, Teams, Discord, WhatsApp,
Google Meet di browser, dan lainnya), notch menganggapnya sedang dalam panggilan.

- **Pil panggilan**: kiri ikon mikrofon (hijau, atau merah dan dicoret saat bisu), tengah jam,
  kanan lama panggilan berwarna hijau. Titik hijau kecil muncul kalau kamera ikut menyala. Pil
  panggilan didahulukan daripada timer dan pomodoro.
- **Musik tetap jalan**: kalau ada lagu yang diputar selama panggilan, pil musik atau pil lirik
  tampil seperti biasa, lengkap dengan sampul lagunya. Panggilan cukup ditandai titik di ujung
  kanan: oranye saat mikrofon menyala, merah saat dibisukan, hijau saat kamera menyala. Lama
  panggilan dan tombol bisunya ada di panel. Saat lagu dijeda, pil panggilan yang penuh muncul
  lagi.
- **Bisu sekali klik**: klik ikon mikrofon di pil untuk membisukan atau menyalakan mikrofon yang
  benar-benar dipakai aplikasi panggilan (bukan sekadar mikrofon default). Panel tidak membuka
  selama kursor ada di ikon itu; klik bagian pil lainnya tetap membuka panel.
- **Kartu panggilan di Beranda** menggantikan jam: nama aplikasi, lama panggilan, status
  mikrofon dan kamera, serta tombol Bisukan/Nyalakan yang besar. Kalau ada pemutar musik
  (diputar atau dijeda), pemutar dan kolom liriknya tetap tampil, dan panggilannya pindah ke
  kepala pemutar: titik hijau, "Discord · 12:34", dan tombol bisu kecil di samping gelombang
  musik. Dengan begitu lagu tetap bisa diputar atau dijeda dari panel. Di lock screen, nama
  aplikasinya disembunyikan.
- **Win+Alt+S** membisukan atau menyalakan mikrofon dari mana saja, disertai kabar singkat
  "Mikrofon dibisukan" atau "Mikrofon menyala" (tetap muncul saat senyap). Tanpa panggilan,
  pintasan ini memakai mikrofon default. Pintasan ini ikut sakelar Pintasan bersama Win+Alt+N.
  Awalnya direncanakan Win+Alt+M, tetapi tombol itu sudah dipakai Xbox Game Bar, dan Win+Alt+K
  dipakai Windows sendiri.
- **Kabar selesai**: "Panggilan selesai · 42 menit" setelah mikrofon dilepas lebih dari 5 detik.
  Kalau aplikasi memakai mikrofon lagi dalam 5 detik itu, panggilannya dianggap masih sama.
- **Aplikasi perantara diabaikan**: NVIDIA Broadcast, RTX Voice, Voicemeeter, Elgato Wave Link,
  Krisp, serta Voice Access dan Live Captions bawaan Windows selalu memegang mikrofon, jadi tidak
  dihitung sebagai panggilan.
- **Abaikan aplikasi tertentu**: Pengaturan → Kabar menampilkan aplikasi yang pernah memakai
  mikrofon (paling banyak 12, disimpan di `panggilan.txt`). Matikan sakelarnya, misal untuk
  Discord yang dibiarkan di voice channel sambil main game, maka pil panggilan tidak muncul
  untuk aplikasi itu. Kalau dimatikan di tengah panggilan, pilnya langsung hilang tanpa kabar
  selesai.
- **Sakelar** "Panggilan aktif" di Pengaturan → Kabar, menyala secara default. Mode rapat tidak
  berubah.
- `cargo test preview_states` menggambar pil, kartu Beranda, dan kabarnya ke `target/preview/`
  (`pill_call*`, `home_call*`, `hud_mic`, `hud_call_ended`).

## Perbaikan setelah Tahap 12

- **Lirik Spotify lebih andal**: saat lagu berganti, Spotify mengirim judul baru lebih dulu dan
  durasinya sesaat kemudian. Notch dulu langsung mencari lirik dengan durasi lagu sebelumnya,
  sehingga pencocokan persis di LRCLIB meleset dan pencarian cadangan dipakai. Sekarang notch
  menunggu 1 detik sesudah lagu berganti, jadi durasinya sudah benar.
- **Gangguan server bukan berarti lirik tidak ada**: kalau server lirik error (misal pencarian
  LRCLIB membalas 503, terlalu banyak permintaan, atau internet putus), kolom lirik menampilkan
  "Server lirik sibuk, mencoba lagi…". Notch mencoba lagi otomatis setelah 30 detik, 2 menit,
  lalu 5 menit selama lagunya masih sama. "Lirik tidak tersedia" hanya muncul kalau liriknya
  memang tidak ditemukan.
- **Klik untuk mencoba lagi**: keterangan "Lirik tidak tersedia" atau "Server lirik sibuk" di
  kolom lirik bisa diklik untuk langsung mencari liriknya lagi.
- **Jam pil panggilan di tengah**: jam di pil panggilan dulu bergeser ke kiri karena memberi
  ruang untuk lama panggilan. Sekarang jamnya tepat di tengah, sama seperti pil biasa.
- **Tetap tampil saat jendela penuh**: sakelar baru di Pengaturan → Tampilan. Kalau dinyalakan,
  notch tidak mengecil jadi garis tipis saat jendela di-maximize, jadi jam, musik, lirik, dan
  panggilan tetap terlihat. Bawaannya mati. Full-screen sungguhan (video, game, presentasi)
  tetap menyembunyikan notch.
