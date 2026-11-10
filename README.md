# SkillBridge OSS

Bahasa Indonesia · [English](README.en.md)

SkillBridge OSS adalah fondasi aplikasi mandiri untuk kelas praktik vokasi. Guru membagikan arahan praktik dan mendaftarkan siswa. Siswa dapat meminta bantuan dan bertukar balasan teks dengan guru kelas di utas pribadi. Tampilan memperbarui data setiap lima detik.

| Cakupan | Status |
| --- | --- |
| Akun, arahan kelas, pendaftaran siswa, tiket bantuan dan balasan | Dapat dijalankan untuk pengembangan dan evaluasi lokal |
| Veyon | Adapter opsional pada komputer guru. Nonaktif secara bawaan. Lihat [panduan lab](docs/veyon.md). |
| SkillPassport, portofolio hasil asesmen dan kredensial digital | [Usulan desain](docs/skillpassport.md) |

Repositori ini belum memuat bukti penggunaan di sekolah, pengguna aktif, sertifikasi atau akreditasi.

## Jalankan secara lokal

Siapkan Docker Engine 28+ dan Docker Compose v2. Salin `.env.example` ke `.env`, lalu isi `POSTGRES_PASSWORD` dengan rahasia pengembangan yang unik. Gunakan huruf, angka, garis bawah atau tanda hubung.

```sh
docker compose up --build --wait
docker compose run --rm api demo
```

Buka [http://127.0.0.1:8080](http://127.0.0.1:8080). Perintah demo membuat satu kelas fiktif dan menampilkan kata sandi acak untuk `guru@example.test` dan `siswa@example.test`. Masuk sebagai siswa, ajukan bantuan, lalu masuk sebagai guru di jendela atau profil browser terpisah untuk membalas. Siswa dapat membalas di utas yang sama. Kedua peserta dapat menutup permintaan.

Pengelola juga dapat membuat akun satu per satu. Perintah berikut meminta kata sandi secara interaktif. Guru kemudian membuat kelas dan mendaftarkan akun siswa melalui email.

```sh
docker compose run --rm api account instructor guru@example.test "Nama Guru"
docker compose run --rm api account student siswa@example.test "Nama Siswa"
docker compose down
```

Gunakan alamat email berbeda jika akun demo sudah dibuat. `docker compose down` menyimpan volume PostgreSQL. Hanya port web pada komputer lokal yang dipublikasikan. Untuk pengembangan dan pengujian di luar Compose, lihat [CONTRIBUTING](CONTRIBUTING.md).

## Dokumentasi

- [Arsitektur, peran dan batas akses](docs/architecture.md)
- [Konfigurasi lab Veyon dan batas lisensi](docs/veyon.md)
- [Rancangan SkillPassport](docs/skillpassport.md) dan [roadmap](docs/roadmap.md)
- [Panduan kontribusi](CONTRIBUTING.md) dan [pelaporan keamanan](SECURITY.md)

Kode SkillBridge OSS memakai [Lisensi MIT](LICENSE). Veyon dipasang secara terpisah dan memiliki [ketentuan lisensinya sendiri](https://github.com/veyon/veyon/blob/main/COPYING).
