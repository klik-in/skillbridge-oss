import { randomUUID } from "node:crypto";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

async function signIn(page: Page, email: string, password: string) {
  await page.goto("/");
  await page.getByLabel("Email", { exact: true }).fill(email);
  await page.getByLabel("Kata sandi").fill(password);
  await page.getByRole("button", { name: "Masuk", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Kelas saya" })).toBeVisible();
  await page.getByRole("button", { name: /Praktik jaringan komputer/ }).click();
}

test("student revises practice work after teacher feedback and receives an assessment", async ({ browser }, info) => {
  test.setTimeout(90_000);
  const teacherPassword = process.env.SKILLBRIDGE_TEACHER_PASSWORD;
  const studentPassword = process.env.SKILLBRIDGE_STUDENT_PASSWORD;
  if (!teacherPassword || !studentPassword) throw new Error("Seed the demo accounts and set the test passwords.");
  const teacher = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  const student = await browser.newPage({ viewport: { width: 375, height: 900 } });
  const title = `Praktik koneksi ${randomUUID().slice(0, 8)}`;
  try {
    await Promise.all([
      signIn(teacher, "guru@example.test", teacherPassword),
      signIn(student, "siswa@example.test", studentPassword),
    ]);
    await teacher.getByText("Buat tugas praktik", { exact: true }).click();
    await teacher.getByLabel("Judul tugas").fill(title);
    await teacher.getByLabel("Instruksi tugas").fill("Bangun koneksi dua komputer dan lampirkan hasil pengujian.");
    await teacher.getByLabel("Kompetensi 1", { exact: true }).fill("Konfigurasi jaringan");
    await teacher.getByLabel("Poin maksimum 1", { exact: true }).fill("60");
    await teacher.getByLabel("Kompetensi 2 · opsional", { exact: true }).fill("Pengujian koneksi");
    await teacher.getByLabel("Poin maksimum 2", { exact: true }).fill("40");
    for (const width of [375, 768, 1280]) {
      await teacher.setViewportSize({ width, height: 900 });
      expect(await teacher.evaluate(() => document.documentElement.scrollWidth)).toBe(width);
      if (process.env.SKILLBRIDGE_QA_CAPTURE === "1") {
        await teacher.locator(".practice-module").screenshot({ path: info.outputPath(`teacher-create-${width}.png`) });
      }
    }
    await teacher.getByRole("button", { name: "Terbitkan tugas" }).click();
    await expect(teacher.getByRole("heading", { name: title, exact: true })).toBeVisible();

    const taskButton = student.getByRole("button", { name: new RegExp(title) });
    await expect(taskButton).toBeVisible({ timeout: 10_000 });
    await taskButton.click();
    await student.getByLabel("Jawaban atau catatan hasil").fill("Konfigurasi awal sudah selesai.");

    // A transient refresh failure must keep the student's unsent answer.
    await student.route("**/api/assignments/*", async (route) => {
      if (route.request().method() === "GET") {
        await route.fulfill({ status: 503, json: { error: "Layanan sedang diperiksa." } });
      } else {
        await route.continue();
      }
    });
    await expect(student.getByRole("alert")).toBeVisible({ timeout: 10_000 });
    await expect(student.getByLabel("Jawaban atau catatan hasil")).toHaveValue("Konfigurasi awal sudah selesai.");
    await student.unroute("**/api/assignments/*");
    await student.getByLabel("Tautan hasil · opsional").fill("https://example.test/hasil-praktik");
    await student.getByRole("button", { name: "Kumpulkan hasil" }).click();
    await expect(student.getByRole("article", { name: "Siswa Demo · Versi 1" })).toBeVisible();
    await expect(student.getByRole("button", { name: "Kumpulkan hasil" })).toHaveCount(0);

    const first = teacher.getByRole("article", { name: "Siswa Demo · Versi 1" });
    await expect(first).toBeVisible({ timeout: 10_000 });
    await first.getByLabel("Konfigurasi jaringan · maksimum 60", { exact: true }).fill("50");
    await first.getByLabel("Pengujian koneksi · maksimum 40", { exact: true }).fill("20");
    await first.getByLabel("Umpan balik").fill("Tambahkan bukti hasil ping.");
    await first.getByRole("button", { name: "Simpan penilaian" }).click();
    await expect(student.getByRole("button", { name: "Kirim revisi" })).toBeVisible({ timeout: 10_000 });
    await expect(student.getByText("Tambahkan bukti hasil ping.")).toBeVisible();
    if (process.env.SKILLBRIDGE_QA_CAPTURE === "1") await student.screenshot({ path: info.outputPath("student-revision-mobile.png"), fullPage: true });

    await student.getByLabel("Jawaban atau catatan hasil").fill("Hasil ping kedua komputer sudah berhasil.");
    await student.getByRole("button", { name: "Kirim revisi" }).click();
    const second = teacher.getByRole("article", { name: "Siswa Demo · Versi 2" });
    await expect(second).toBeVisible({ timeout: 10_000 });
    await second.getByLabel("Konfigurasi jaringan · maksimum 60", { exact: true }).fill("60");
    await second.getByLabel("Pengujian koneksi · maksimum 40", { exact: true }).fill("40");
    await second.getByLabel("Hasil penilaian").selectOption("accepted");
    await second.getByLabel("Umpan balik").fill("Konfigurasi dan pengujian sudah lengkap.");
    if (process.env.SKILLBRIDGE_QA_CAPTURE === "1") await teacher.screenshot({ path: info.outputPath("teacher-review-desktop.png"), fullPage: true });
    await second.getByRole("button", { name: "Simpan penilaian" }).click();
    const accepted = student.getByRole("article", { name: "Siswa Demo · Versi 2" });
    await expect(accepted.getByText("Diterima", { exact: true })).toBeVisible({ timeout: 10_000 });
    await expect(accepted.getByText("Nilai 100 / 100", { exact: true })).toBeVisible();
    await expect(student.getByRole("article", { name: "Siswa Demo · Versi 1" })).toBeVisible();
    await expect(student.getByRole("button", { name: "Kirim revisi" })).toHaveCount(0);

    for (const width of [375, 768, 1280]) {
      await student.setViewportSize({ width, height: 900 });
      expect(await student.evaluate(() => document.documentElement.scrollWidth)).toBe(width);
      if (process.env.SKILLBRIDGE_QA_CAPTURE === "1") {
        await student.screenshot({ path: info.outputPath(`student-accepted-${width}.png`), fullPage: true });
      }
    }
  } finally {
    await teacher.close();
    await student.close();
  }
});
