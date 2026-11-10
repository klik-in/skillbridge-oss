import { randomUUID } from "node:crypto";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

async function signIn(page: Page, email: string, password: string) {
  await page.goto("/");
  await page.getByLabel("Email", { exact: true }).fill(email);
  await page.getByLabel("Kata sandi").fill(password);
  await page.getByRole("button", { name: "Masuk", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Kelas saya" })).toBeVisible();
}

function demoPasswords() {
  const teacherPassword = process.env.SKILLBRIDGE_TEACHER_PASSWORD;
  const studentPassword = process.env.SKILLBRIDGE_STUDENT_PASSWORD;
  if (!teacherPassword || !studentPassword) throw new Error("Seed the demo accounts and set the two test passwords.");
  return { teacherPassword, studentPassword };
}

test("student and teacher exchange replies in a practice class", async ({ browser }, info) => {
  const { teacherPassword, studentPassword } = demoPasswords();
  const teacherContext = await browser.newContext();
  const studentContext = await browser.newContext({ viewport: { width: 390, height: 844 } });
  try {
    const teacher = await teacherContext.newPage();
    const student = await studentContext.newPage();
    await Promise.all([
      signIn(teacher, "guru@example.test", teacherPassword),
      signIn(student, "siswa@example.test", studentPassword),
    ]);
    const demoClass = { name: /Praktik jaringan komputer/ };
    await teacher.getByRole("button", demoClass).click();
    await student.getByRole("button", demoClass).click();
    await expect(student.getByRole("heading", { name: "Praktik jaringan komputer" })).toBeVisible();
    const subject = `Tes bantuan ${randomUUID()}`;
    await student.getByLabel("Apa kendalanya?").fill(subject);
    await student.getByRole("button", { name: "Ajukan bantuan" }).click();

    const incoming = teacher.getByRole("button", { name: new RegExp(subject) });
    await expect(incoming).toBeVisible({ timeout: 10_000 });
    await incoming.click();
    await teacher.getByLabel("Balasan").fill("Periksa sambungan kabel.");
    await teacher.getByRole("button", { name: "Kirim pesan" }).click();
    await expect(student.getByText("Periksa sambungan kabel.")).toBeVisible({ timeout: 10_000 });

    await student.getByLabel("Balasan").fill("Sambungan sudah berfungsi.");
    await student.getByRole("button", { name: "Kirim pesan" }).click();
    await expect(teacher.getByText("Sambungan sudah berfungsi.")).toBeVisible({ timeout: 10_000 });

    if (process.env.SKILLBRIDGE_QA_CAPTURE === "1") {
      await teacher.screenshot({ path: info.outputPath("teacher-desktop.png"), fullPage: true });
      await student.screenshot({ path: info.outputPath("student-mobile.png"), fullPage: true });
    }
    expect(await student.evaluate(() => document.documentElement.scrollWidth)).toBe(390);
    await teacher.getByRole("button", { name: "Tutup permintaan" }).click();
    await expect(student.getByRole("button", { name: "Kirim pesan" })).toHaveCount(0, { timeout: 10_000 });

    await student.getByRole("button", { name: "Keluar" }).click();
    await expect(student.getByRole("heading", { name: "Masuk ke SkillBridge" })).toBeVisible();
    if (process.env.SKILLBRIDGE_QA_CAPTURE === "1") {
      await student.screenshot({ path: info.outputPath("login-mobile.png"), fullPage: true });
      await teacher.goto("/");
      await teacher.screenshot({ path: info.outputPath("login-desktop.png"), fullPage: true });
    }
  } finally {
    await teacherContext.close();
    await studentContext.close();
  }
});

test("teacher creates a class and enrolls a student by email", async ({ browser }) => {
  const { teacherPassword, studentPassword } = demoPasswords();
  const teacher = await browser.newPage();
  const student = await browser.newPage();
  try {
    await signIn(teacher, "guru@example.test", teacherPassword);
    const title = `Kelas uji ${randomUUID()}`;
    await teacher.getByText("Buat kelas praktik", { exact: true }).click();
    await teacher.getByLabel("Judul", { exact: true }).fill(title);
    await teacher.getByLabel("Arahan praktik", { exact: true }).fill("Uji koneksi jaringan lokal.");
    await teacher.getByRole("button", { name: "Simpan kelas" }).click();
    await expect(teacher.getByRole("heading", { name: title })).toBeVisible();
    await teacher.getByLabel("Email akun siswa").fill("siswa@example.test");
    await teacher.getByRole("button", { name: "Daftarkan" }).click();
    await expect(teacher.getByRole("status")).toContainText("Siswa terdaftar di kelas.");

    await signIn(student, "siswa@example.test", studentPassword);
    await student.getByRole("button", { name: new RegExp(title) }).click();
    await expect(student.getByText("Uji koneksi jaringan lokal.")).toBeVisible();
  } finally {
    await teacher.close();
    await student.close();
  }
});
