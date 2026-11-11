import { useState } from "react";
import * as z from "zod/mini";
import { createdSchema, read, write } from "./api";
import type { Session } from "./api";
import { ActionForm, Notice, field } from "./forms";
import { useResource } from "./resource";

const criterionSchema = z.object({ label: z.string(), max_score: z.number() });
const assignmentSchema = z.object({
  id: z.uuid(), class_id: z.uuid(), title: z.string(), instructions: z.string(),
  rubric: z.array(criterionSchema), due_at: z.nullable(z.string()), created_at: z.string(),
});
const reviewSchema = z.object({
  scores: z.array(z.number()), outcome: z.enum(["revision_requested", "accepted"]),
  feedback: z.string(), reviewer_name: z.string(), reviewed_at: z.string(),
});
const submissionSchema = z.object({
  id: z.uuid(), student_id: z.uuid(), student_name: z.string(), attempt: z.number(),
  body: z.string(), artifact_url: z.nullable(z.string()), submitted_at: z.string(),
  late: z.boolean(), review: z.nullable(reviewSchema),
});
const assignmentsSchema = z.array(assignmentSchema);
const detailSchema = z.object({ assignment: assignmentSchema, submissions: z.array(submissionSchema) });
type Assignment = z.infer<typeof assignmentSchema>;
type Submission = z.infer<typeof submissionSchema>;
const time = new Intl.DateTimeFormat("id-ID", { dateStyle: "medium", timeStyle: "short" });

function CreateAssignment({ classId, token, onCreated }: Readonly<{ classId: string; token: string; onCreated: (id: string) => void }>) {
  const [expanded, setExpanded] = useState(false);
  return (
    <details className="new-class" open={expanded} onToggle={(event) => setExpanded(event.currentTarget.open)}>
      <summary>Buat tugas praktik</summary>
      <ActionForm label="Terbitkan tugas" perform={async (data) => {
        const rubric = [1, 2, 3, 4, 5].flatMap((index) => {
          const label = field(data, `criterion-${index}`).trim();
          return label ? [{ label, max_score: Number(field(data, `points-${index}`)) }] : [];
        });
        const due = field(data, "due");
        const assignment = await read(`/classes/${classId}/assignments`, assignmentSchema, {
          token, method: "POST",
          body: {
            title: field(data, "title"), instructions: field(data, "instructions"), rubric,
            due_at: due ? new Date(due).toISOString() : null,
          },
        });
        onCreated(assignment.id);
        setExpanded(false);
      }}>
        <label>Judul tugas<input name="title" required maxLength={120} /></label>
        <label>Instruksi tugas<textarea name="instructions" required maxLength={4000} rows={4} /></label>
        <label>Tenggat waktu · opsional<input name="due" type="datetime-local" /></label>
        <p className="muted">Tenggat mengikuti waktu perangkat Anda. Kiriman lewat tenggat ditandai terlambat.</p>
        <fieldset className="rubric-fields">
          <legend>Rubrik penilaian</legend>
          <p className="muted">Isi 1–5 kompetensi dengan total maksimum 100 poin. Rubrik disimpan saat tugas diterbitkan.</p>
          {[1, 2, 3, 4, 5].map((index) => (
            <div className="rubric-row" key={index}>
              <label>Kompetensi {index}{index > 1 ? " · opsional" : ""}
                <input name={`criterion-${index}`} maxLength={120} required={index === 1} />
              </label>
              <label>Poin maksimum {index}
                <input name={`points-${index}`} type="number" min={1} max={100} step={1} defaultValue={index === 1 ? 100 : 10} required />
              </label>
            </div>
          ))}
        </fieldset>
      </ActionForm>
    </details>
  );
}

export function Practice({ classId, session }: Readonly<{ classId: string; session: Session }>) {
  const [revision, setRevision] = useState(0);
  const [selected, select] = useState<string | null>(null);
  const tasks = useResource(`/classes/${classId}/assignments`, assignmentsSchema, session.token, revision, true);
  const active = tasks.data?.find((task) => task.id === selected);

  return (
    <section className="panel practice-module" aria-labelledby="practice-heading">
      <div className="section-heading"><h3 id="practice-heading">Tugas praktik</h3><span className="eyebrow">Belajar &amp; buktikan</span></div>
      {session.account.role === "instructor" && <CreateAssignment classId={classId} token={session.token} onCreated={(id) => {
        select(id);
        setRevision((value) => value + 1);
      }} />}
      <Notice message={tasks.error} />
      {!tasks.data && !tasks.error && <p className="muted">Memuat tugas…</p>}
      {tasks.data?.length === 0 && <p className="muted">Belum ada tugas praktik di kelas ini.</p>}
      <ul className="ticket-list">
        {tasks.data?.map((task) => (
          <li key={task.id}><button type="button" className="ticket-button" aria-pressed={task.id === active?.id}
            onClick={() => select(task.id)}>
            <span><strong>{task.title}</strong><small>{task.due_at ? `Tenggat ${time.format(new Date(task.due_at))}` : "Tanpa tenggat"}</small></span>
            <span aria-hidden="true">→</span>
          </button></li>
        ))}
      </ul>
      {active && <AssignmentDetail key={active.id} id={active.id} session={session} />}
    </section>
  );
}

function Assessment({ submission, assignment, token, onSaved }: Readonly<{ submission: Submission; assignment: Assignment; token: string; onSaved: () => void }>) {
  return (
    <ActionForm label="Simpan penilaian" perform={async (data) => {
      await write(`/submissions/${submission.id}/review`, {
        token, method: "POST",
        body: {
          scores: assignment.rubric.map((_, index) => Number(field(data, `score-${index}`))),
          outcome: field(data, "outcome"), feedback: field(data, "feedback"),
        },
      });
      onSaved();
    }}>
      <fieldset className="rubric-fields">
        <legend>Nilai per kompetensi</legend>
        {assignment.rubric.map((criterion, index) => (
          <label key={criterion.label}>{criterion.label} · maksimum {criterion.max_score}
            <input name={`score-${index}`} type="number" min={0} max={criterion.max_score} step={1} required />
          </label>
        ))}
      </fieldset>
      <label>Hasil penilaian<select name="outcome" defaultValue="revision_requested">
        <option value="revision_requested">Perlu revisi</option>
        <option value="accepted">Diterima</option>
      </select></label>
      <label>Umpan balik<textarea name="feedback" rows={3} maxLength={2000} required /></label>
    </ActionForm>
  );
}

function SubmissionCard({ submission, assignment, session, latest, onSaved }: Readonly<{
  submission: Submission; assignment: Assignment; session: Session; latest: boolean; onSaved: () => void;
}>) {
  const review = submission.review;
  const status = review?.outcome ?? "pending";
  const labels = { pending: "Menunggu penilaian", revision_requested: "Perlu revisi", accepted: "Diterima" };

  return (
    <article className="submission-card" aria-label={`${submission.student_name} · Versi ${submission.attempt}`}>
      <div className="section-heading">
        <h4>{submission.student_name} · Versi {submission.attempt}</h4>
        <span className={`status ${status}`}>{labels[status]}</span>
      </div>
      <p className="muted metadata"><time dateTime={submission.submitted_at}>{time.format(new Date(submission.submitted_at))}</time>
        {submission.late && <span className="status late">Terlambat</span>}
      </p>
      <p className="multiline">{submission.body}</p>
      {submission.artifact_url && <p><a href={submission.artifact_url} target="_blank" rel="noopener noreferrer">Hasil praktik · {submission.artifact_url}</a></p>}
      {review && <div className="assessment-result">
        <p><strong>Nilai {review.scores.reduce((total, score) => total + score, 0)} / {assignment.rubric.reduce((total, criterion) => total + criterion.max_score, 0)}</strong></p>
        <ul>{assignment.rubric.map((criterion, index) => (
          <li key={criterion.label}>{criterion.label} · {review.scores[index]} / {criterion.max_score}</li>
        ))}</ul>
        <p className="multiline">{review.feedback}</p>
        <p className="muted metadata">{review.reviewer_name} · {time.format(new Date(review.reviewed_at))}</p>
      </div>}
      {latest && !review && session.account.role === "instructor" &&
        <Assessment submission={submission} assignment={assignment} token={session.token} onSaved={onSaved} />}
    </article>
  );
}

function AssignmentDetail({ id, session }: Readonly<{ id: string; session: Session }>) {
  const [revision, setRevision] = useState(0);
  const result = useResource(`/assignments/${id}`, detailSchema, session.token, revision, true);
  const detail = result.data;
  const latestByStudent = new Map<string, Submission>();
  for (const item of detail?.submissions ?? []) {
    const previous = latestByStudent.get(item.student_id);
    if (!previous || item.attempt > previous.attempt) latestByStudent.set(item.student_id, item);
  }
  const ownLatest = latestByStudent.get(session.account.id);
  const canSubmit = !ownLatest || ownLatest.review?.outcome === "revision_requested";
  const refresh = () => setRevision((value) => value + 1);

  return (
    <div className="assignment-detail">
      <Notice message={result.error} />
      {!detail && !result.error && <p className="muted">Memuat tugas…</p>}
      {detail && <>
        <h4>{detail.assignment.title}</h4>
        <p className="multiline">{detail.assignment.instructions}</p>
        <div className="rubric-summary">
          <strong>Rubrik kompetensi</strong>
          <ul>{detail.assignment.rubric.map((criterion) => (
            <li key={criterion.label}>{criterion.label} · {criterion.max_score} poin</li>
          ))}</ul>
        </div>
        {session.account.role === "student" && canSubmit && <ActionForm
          label={ownLatest ? "Kirim revisi" : "Kumpulkan hasil"} perform={async (data) => {
            await read(`/assignments/${id}/submissions`, createdSchema, {
              token: session.token, method: "POST",
              body: { body: field(data, "body"), artifact_url: field(data, "artifact") || null },
            });
            refresh();
          }}>
          <label>Jawaban atau catatan hasil<textarea name="body" rows={4} maxLength={6000} required /></label>
          <label>Tautan hasil · opsional<input name="artifact" type="url" pattern="https://.*" maxLength={2048} placeholder="https://" /></label>
        </ActionForm>}
        <h4>Riwayat pengumpulan</h4>
        {detail.submissions.length === 0 && <p className="muted">Belum ada hasil yang dikumpulkan.</p>}
        {detail.submissions.map((submission) => (
          <SubmissionCard key={submission.id} submission={submission} assignment={detail.assignment} session={session}
            latest={latestByStudent.get(submission.student_id)?.id === submission.id}
            onSaved={refresh} />
        ))}
      </>}
    </div>
  );
}
