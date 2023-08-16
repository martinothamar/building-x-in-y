CREATE TABLE IF NOT EXISTS todos
(
    id          BLOB    PRIMARY KEY NOT NULL,
    ordering    INTEGER             NOT NULL,
    title       TEXT                NOT NULL,
    description TEXT                NOT NULL,
    done        BOOLEAN             NOT NULL DEFAULT 0
);
