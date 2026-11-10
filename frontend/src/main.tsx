import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import * as z from "zod/mini";
import {
  ApiError, classSchema, classroomSchema, createdSchema, errorText,
  messageSchema, read, sessionSchema, ticketSchema, write,
} from "./api";
import type { Classroom, Decoder, Session, Ticket } from "./api";
import { ActionForm, Notice, field } from "./forms";
import "./style.css";

const classesSchema = z.array(classSchema);
const ticketsSchema = z.array(ticketSchema);
const messagesSchema = z.array(messageSchema);
const time = new Intl.DateTimeFormat("id-ID", { dateStyle: "medium", timeStyle: "short" });

type Load<T> = Readonly<{ data: T | null; error: string | null }>;

function useResource<T>(path: string, schema: Decoder<T>, token: string, revision: number, poll = false): Load<T> {
  const [resource, setResource] = useState<Load<T>>({ data: null, error: null });
  useEffect(() => {
    const controller = new AbortController();
    let timer: number | undefined;

    async function refresh() {
      try {
        const data = await read(path, schema, { token, signal: controller.signal });
        if (!controller.signal.aborted) setResource({ data, error: null });
      } catch (error) {
        if (!controller.signal.aborted) setResource({ data: null, error: errorText(error) });
      }
      if (poll && !controller.signal.aborted) timer = window.setTimeout(refresh, 5000);
    }

    void refresh();
    return () => {
      controller.abort();
      window.clearTimeout(timer);
    };
  }, [path, schema, token, revision, poll]);
  return resource;
}

function Brand() {
  return (
    <div className="brand">
      <img src="/mark.svg" alt="" width="40" height="40" />
      <div><strong>SkillBridge <small>OSS</small></strong><span>Ruang praktik vokasi</span></div>
    </div>
  );
}

function SignIn({ onSignIn }: Readonly<{ onSignIn: (session: Session) => void }>) {
  return (
    <>
      <header className="topbar"><Brand /></header>
      <main className="sign-in">
        <div className="welcome">
          <span className="eyebrow">Pendidikan vokasi</span>
          <h1>Ruang untuk belajar lewat praktik.</h1>
          <p>Buka arahan kelas, ajukan bantuan saat menemui kendala, dan lanjutkan percakapan dengan guru di satu tempat.</p>
          <svg className="bridge-art" viewBox="0 0 480 190" aria-hidden="true">
            <path d="M10 160h460M38 160v-45l202-92 202 92v45M97 160v-64m71 64V64m72 96V23m72 137V64m71 96V96" />
            <path d="M38 115h404M10 178h460" opacity=".35" />
          </svg>
        </div>
        <section className="panel sign-in-panel" aria-labelledby="login-title">
          <span className="eyebrow">Akses kelas</span>
          <h2 id="login-title">Masuk ke SkillBridge</h2>
          <p className="muted">Gunakan akun yang diberikan oleh pengelola.</p>
          <ActionForm label="Masuk" reset={false} perform={async (data) => {
            onSignIn(await read("/session", sessionSchema, {
              method: "POST", body: { email: field(data, "email"), password: field(data, "password") },
            }));
          }}>
            <label>Email<input name="email" type="email" autoComplete="username" maxLength={254} required /></label>
            <label>Kata sandi<input name="password" type="password" autoComplete="current-password" maxLength={256} required /></label>
          </ActionForm>
        </section>
      </main>
    </>
  );
}

function Workspace({ session, onSignOut }: Readonly<{ session: Session; onSignOut: () => void }>) {
  const { account, token } = session;
  const [selected, select] = useState<string | null>(null);
  const [revision, setRevision] = useState(0);
  const classes = useResource("/classes", classesSchema, token, revision, true);
  const active = classes.data?.find((item) => item.id === selected) ?? classes.data?.[0];

  return (
    <>
      <header className="topbar">
        <Brand />
        <div className="profile">
          <div className="profile-name"><strong>{account.name}</strong><span>{account.role === "instructor" ? "Guru" : "Siswa"}</span></div>
          <ActionForm label="Keluar" secondary perform={async () => {
            try {
              await write("/session", { method: "DELETE", token });
            } catch (error) {
              if (!(error instanceof ApiError && error.status === 401)) throw error;
            }
            onSignOut();
          }} />
        </div>
      </header>
      <main className="workspace">
        <aside className="sidebar" aria-label="Kelas saya">
          <span className="eyebrow">Pembelajaran</span>
          <h1>Kelas saya</h1>
          <Notice message={classes.error} />
          {classes.data === null && !classes.error && <p className="muted">Memuat kelas…</p>}
          {classes.data?.length === 0 && <p className="muted">Belum ada kelas untuk akun ini.</p>}
          <nav aria-label="Daftar kelas">
            {classes.data?.map((item) => (
              <button type="button" key={item.id} className="class-button"
                aria-current={item.id === active?.id ? "true" : undefined}
                onClick={() => select(item.id)}>
                <span className="class-icon" aria-hidden="true">▤</span>
                <span><strong>{item.title}</strong><small>{item.instructor_name}</small></span>
              </button>
            ))}
          </nav>
          {account.role === "instructor" && (
            <details className="new-class">
              <summary>Buat kelas praktik</summary>
              <ActionForm label="Simpan kelas" perform={async (data) => {
                const item = await read("/classes", classSchema, {
                  token, method: "POST", body: { title: field(data, "title"), brief: field(data, "brief") },
                });
                select(item.id);
                setRevision((value) => value + 1);
              }}>
                <label>Judul<input name="title" maxLength={120} required /></label>
                <label>Arahan praktik<textarea name="brief" maxLength={2000} rows={4} required /></label>
              </ActionForm>
            </details>
          )}
        </aside>
        {active ? <ClassPanel key={active.id} classId={active.id} session={session} /> : (
          <section className="panel empty-workspace"><span className="eyebrow">Ruang praktik</span>
            <h2>Kelas praktik akan tampil di sini.</h2>
            <p className="muted">{account.role === "instructor" ? "Buat kelas untuk membagikan arahan dan menerima permintaan bantuan." : "Guru dapat mendaftarkan Anda ke kelas dengan alamat email akun Anda."}</p>
          </section>
        )}
      </main>
    </>
  );
}

function ClassPanel({ classId, session }: Readonly<{ classId: string; session: Session }>) {
  const path = `/classes/${classId}`;
  const { token, account } = session;
  const [selected, select] = useState<string | null>(null);
  const [revision, setRevision] = useState(0);
  const [enrolled, setEnrolled] = useState(false);
  const classroom = useResource(path, classroomSchema, token, revision, true);
  const tickets = useResource(`${path}/help`, ticketsSchema, token, revision, true);
  const active = tickets.data?.find((ticket) => ticket.id === selected);
  const refresh = () => setRevision((value) => value + 1);
  const item = classroom.data;

  return (
    <div className="class-content">
      <Notice message={classroom.error} />
      {item && (
        <>
          <section className="class-hero">
            <span className="eyebrow">Kelas praktik</span>
            <h2>{item.title}</h2>
            <p>Guru · {item.instructor_name}</p>
          </section>
          <div className="class-grid">
            <div className="column">
              <section className="panel">
                <h3>Arahan praktik</h3>
                <p className="multiline">{item.brief}</p>
                <p className="muted metadata">ID kelas <code>{item.id}</code></p>
              </section>
              {account.role === "instructor" ? (
                <section className="panel">
                  <h3>Daftarkan siswa</h3>
                  <ActionForm label="Daftarkan" perform={async (data) => {
                    setEnrolled(false);
                    await write(`${path}/enrollments`, { token, method: "POST", body: { email: field(data, "email") } });
                    setEnrolled(true);
                  }}>
                    <label>Email akun siswa<input name="email" type="email" maxLength={254} required /></label>
                  </ActionForm>
                  {enrolled && <p className="success" role="status">Siswa terdaftar di kelas.</p>}
                </section>
              ) : <NewTicket classroom={item} token={token} onCreated={(id) => { select(id); refresh(); }} />}
            </div>
            <section className="panel tickets">
              <div className="section-heading"><h3>Permintaan bantuan</h3><span className="eyebrow">Terbaru</span></div>
              <Notice message={tickets.error} />
              {!tickets.data && !tickets.error && <p className="muted">Memuat permintaan…</p>}
              {tickets.data?.length === 0 && <p className="muted">Belum ada permintaan bantuan.</p>}
              <ul className="ticket-list">
                {tickets.data?.map((ticket) => (
                  <li key={ticket.id}>
                    <button type="button" aria-pressed={active?.id === ticket.id} className="ticket-button" onClick={() => select(ticket.id)}>
                      <span><strong>{ticket.subject}</strong><small>{ticket.student_name} · {time.format(new Date(ticket.created_at))}</small></span>
                      <span className={`status ${ticket.status}`}>{ticket.status === "open" ? "Terbuka" : "Ditutup"}</span>
                    </button>
                  </li>
                ))}
              </ul>
            </section>
          </div>
          {active && <Conversation key={active.id} ticket={active} session={session} onClosed={refresh} />}
        </>
      )}
      {!item && !classroom.error && <p className="muted">Memuat kelas…</p>}
    </div>
  );
}

function NewTicket({ classroom, token, onCreated }: Readonly<{ classroom: Classroom; token: string; onCreated: (id: string) => void }>) {
  return (
    <section className="panel">
      <h3>Minta bantuan</h3>
      <ActionForm label="Ajukan bantuan" perform={async (data) => {
        const result = await read(`/classes/${classroom.id}/help`, createdSchema, {
          token, method: "POST", body: { subject: field(data, "subject"), consent: data.has("consent") },
        });
        onCreated(result.id);
      }}>
        <label>Apa kendalanya?<textarea name="subject" maxLength={200} rows={3} required /></label>
        {classroom.seat && <label className="check-label"><input type="checkbox" name="consent" />
          <span>Saya mengizinkan guru mengendalikan komputer lab {classroom.seat.label} melalui Veyon. Izin berlaku sekali selama 10 menit. Tutup tiket untuk membatalkan izin yang belum dipakai.</span>
        </label>}
      </ActionForm>
    </section>
  );
}

function Conversation({ ticket, session, onClosed }: Readonly<{ ticket: Ticket; session: Session; onClosed: () => void }>) {
  const path = `/help/${ticket.id}`;
  const { token } = session;
  const [revision, setRevision] = useState(0);
  const messages = useResource(`${path}/messages`, messagesSchema, token, revision, true);

  return (
    <section className="panel conversation">
      <div className="section-heading"><h3>Percakapan</h3><span className={`status ${ticket.status}`}>{ticket.status === "open" ? "Terbuka" : "Ditutup"}</span></div>
      <p className="conversation-subject">{ticket.subject}</p>
      {ticket.claimed_at && <p className="muted">Izin Veyon digunakan pada {time.format(new Date(ticket.claimed_at))}.</p>}
      {ticket.remote_available && ticket.seat_id && session.account.role === "instructor" && (
        <details className="remote-details">
          <summary>Izin Veyon untuk {ticket.seat_label}</summary>
          <p>Jalankan adapter pada komputer guru dengan konfigurasi lokal sekolah.</p>
          <dl className="metadata"><dt>ID tiket</dt><dd><code>{ticket.id}</code></dd><dt>ID kursi</dt><dd><code>{ticket.seat_id}</code></dd></dl>
        </details>
      )}
      <Notice message={messages.error} />
      {messages.data?.length === 0 && <p className="muted">Belum ada balasan.</p>}
      <ol className="messages">
        {messages.data?.map((message) => (
          <li key={message.id}>
            <div><strong>{message.author_name}</strong><time dateTime={message.created_at}>{time.format(new Date(message.created_at))}</time></div>
            <p className="multiline">{message.body}</p>
          </li>
        ))}
      </ol>
      {ticket.status === "open" && <div className="reply-tools">
        <ActionForm label="Kirim pesan" perform={async (data) => {
          await write(`${path}/messages`, { token, method: "POST", body: { body: field(data, "message") } });
          setRevision((value) => value + 1);
        }}>
          <label>Balasan<textarea name="message" rows={3} maxLength={2000} required /></label>
        </ActionForm>
        <ActionForm label="Tutup permintaan" secondary perform={async () => {
          await write(`${path}/close`, { token, method: "POST" });
          onClosed();
        }} />
      </div>}
    </section>
  );
}

function App() {
  const [session, setSession] = useState<Session | null>(null);
  return session
    ? <Workspace session={session} onSignOut={() => setSession(null)} />
    : <SignIn onSignIn={setSession} />;
}

const mount = document.getElementById("root");
if (mount) createRoot(mount).render(<App />);
