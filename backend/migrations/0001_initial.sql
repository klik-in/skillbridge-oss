CREATE TYPE account_role AS ENUM ('instructor', 'student');
CREATE TYPE help_status AS ENUM ('open', 'closed');

CREATE TABLE accounts (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    email text NOT NULL UNIQUE,
    name text NOT NULL,
    role account_role NOT NULL,
    password_hash text NOT NULL
);

CREATE TABLE sessions (
    digest bytea PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts ON DELETE CASCADE,
    expires_at timestamptz NOT NULL DEFAULT (now() + interval '8 hours')
);
CREATE INDEX sessions_expiry ON sessions (expires_at);

CREATE TABLE classes (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    instructor_id uuid NOT NULL REFERENCES accounts,
    title text NOT NULL,
    brief text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX classes_instructor ON classes (instructor_id, created_at DESC);

CREATE TABLE enrollments (
    class_id uuid NOT NULL REFERENCES classes,
    student_id uuid NOT NULL REFERENCES accounts,
    PRIMARY KEY (class_id, student_id)
);
CREATE INDEX enrollments_student ON enrollments (student_id, class_id);

-- Device IDs identify operator-assigned seats. Network addresses stay on teacher workstations.
CREATE TABLE lab_seats (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    class_id uuid NOT NULL,
    student_id uuid NOT NULL,
    label text NOT NULL,
    FOREIGN KEY (class_id, student_id) REFERENCES enrollments (class_id, student_id),
    UNIQUE (class_id, student_id),
    UNIQUE (class_id, label),
    UNIQUE (id, class_id, student_id)
);

CREATE TABLE help_requests (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    class_id uuid NOT NULL,
    student_id uuid NOT NULL,
    subject text NOT NULL,
    status help_status NOT NULL DEFAULT 'open',
    created_at timestamptz NOT NULL DEFAULT now(),
    seat_id uuid,
    consent_until timestamptz,
    claimed_at timestamptz,
    claimed_by uuid REFERENCES accounts,
    FOREIGN KEY (class_id, student_id) REFERENCES enrollments (class_id, student_id),
    FOREIGN KEY (seat_id, class_id, student_id) REFERENCES lab_seats (id, class_id, student_id),
    CHECK ((seat_id IS NULL) = (consent_until IS NULL)),
    CHECK ((claimed_at IS NULL) = (claimed_by IS NULL)),
    CHECK (claimed_at IS NULL OR seat_id IS NOT NULL)
);
CREATE INDEX help_class ON help_requests (class_id, created_at DESC, id DESC);
CREATE INDEX help_student ON help_requests (class_id, student_id, created_at DESC, id DESC);

CREATE TABLE messages (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    help_id uuid NOT NULL REFERENCES help_requests,
    author_id uuid NOT NULL REFERENCES accounts,
    body text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX messages_help ON messages (help_id, id DESC);
