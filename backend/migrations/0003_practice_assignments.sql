CREATE TABLE assignments (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    class_id uuid NOT NULL REFERENCES classes,
    title text NOT NULL,
    instructions text NOT NULL,
    rubric jsonb NOT NULL CHECK (jsonb_typeof(rubric) = 'array'),
    due_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    UNIQUE (id, class_id)
);
CREATE INDEX assignments_class ON assignments (class_id, created_at DESC, id DESC);

CREATE TABLE submissions (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    assignment_id uuid NOT NULL,
    class_id uuid NOT NULL,
    student_id uuid NOT NULL,
    attempt integer NOT NULL CHECK (attempt > 0),
    body text NOT NULL,
    artifact_url text,
    submitted_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    FOREIGN KEY (assignment_id, class_id) REFERENCES assignments (id, class_id),
    FOREIGN KEY (class_id, student_id) REFERENCES enrollments (class_id, student_id),
    UNIQUE (assignment_id, student_id, attempt)
);
CREATE INDEX submissions_assignment ON submissions (assignment_id, submitted_at DESC, id DESC);

CREATE TYPE assessment_outcome AS ENUM ('revision_requested', 'accepted');
CREATE TABLE submission_reviews (
    submission_id uuid PRIMARY KEY REFERENCES submissions,
    reviewed_by uuid NOT NULL REFERENCES accounts,
    scores jsonb NOT NULL CHECK (jsonb_typeof(scores) = 'array'),
    outcome assessment_outcome NOT NULL,
    feedback text NOT NULL,
    reviewed_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
