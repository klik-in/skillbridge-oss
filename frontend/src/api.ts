import * as z from "zod/mini";

const account = z.object({
  id: z.uuid(),
  name: z.string(),
  role: z.enum(["instructor", "student"]),
});

export const sessionSchema = z.object({ token: z.string(), account });
export type Session = z.infer<typeof sessionSchema>;

export const classSchema = z.object({
  id: z.uuid(),
  title: z.string(),
  brief: z.string(),
  instructor_name: z.string(),
});

export const classroomSchema = z.extend(classSchema, {
  seat: z.nullable(z.object({ id: z.uuid(), label: z.string() })),
});
export type Classroom = z.infer<typeof classroomSchema>;

export const ticketSchema = z.object({
  id: z.uuid(),
  subject: z.string(),
  student_name: z.string(),
  status: z.enum(["open", "closed"]),
  created_at: z.string(),
  seat_id: z.nullable(z.uuid()),
  seat_label: z.nullable(z.string()),
  remote_available: z.boolean(),
  claimed_at: z.nullable(z.string()),
});
export type Ticket = z.infer<typeof ticketSchema>;

export const messageSchema = z.object({
  id: z.number(),
  author_name: z.string(),
  body: z.string(),
  created_at: z.string(),
});

export const createdSchema = z.object({ id: z.uuid() });

const problem = z.object({ error: z.string() });

export class ApiError extends Error {
  readonly name = "ApiError";

  constructor(readonly status: number, detail: string) {
    super(detail);
  }
}

export function errorText(error: unknown): string {
  if (!(error instanceof ApiError)) return "Koneksi ke layanan terputus. Coba lagi.";
  switch (error.status) {
    case 401: return "Email atau kata sandi salah, atau sesi telah berakhir.";
    case 403: return "Peran Anda tidak memiliki akses untuk tindakan ini.";
    case 404: return "Data tidak ditemukan atau tidak dapat diakses.";
    case 429: return "Terlalu banyak percobaan masuk. Coba lagi dalam satu menit.";
    default: return error.message;
  }
}

type Options = Readonly<{
  token?: string;
  method?: "GET" | "POST" | "DELETE";
  body?: object;
  signal?: AbortSignal;
}>;

async function request(path: string, options: Options): Promise<Response> {
  const headers = new Headers();
  if (options.token) headers.set("Authorization", `Bearer ${options.token}`);
  if (options.body) headers.set("Content-Type", "application/json");
  const response = await fetch(`/api${path}`, {
    method: options.method ?? "GET",
    headers,
    body: options.body ? JSON.stringify(options.body) : null,
    signal: options.signal ?? null,
  });
  if (!response.ok) {
    const raw: unknown = await response.json().catch(() => null);
    const parsed = problem.safeParse(raw);
    throw new ApiError(response.status, parsed.success ? parsed.data.error : "Permintaan gagal. Periksa input atau coba lagi.");
  }
  return response;
}

export type Decoder<T> = Readonly<{ parse: (input: unknown) => T }>;

export async function read<T>(path: string, schema: Decoder<T>, options: Options): Promise<T> {
  const response = await request(path, options);
  const data: unknown = await response.json();
  return schema.parse(data);
}

export async function write(path: string, options: Options): Promise<void> {
  await request(path, options);
}
