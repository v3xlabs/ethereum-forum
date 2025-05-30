CREATE TABLE workshop_messages (
    message_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chat_id UUID NOT NULL,
    sender_role TEXT NOT NULL CHECK (sender_role IN ('user', 'assistant', 'system')),
    message TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    parent_message_id UUID,
    FOREIGN KEY (parent_message_id) REFERENCES workshop_messages(message_id) ON DELETE SET NULL
);

CREATE TABLE workshop_chats (
    chat_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id INT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ,
    summary TEXT,
    last_message_id UUID,
    FOREIGN KEY (last_message_id) REFERENCES workshop_messages(message_id)
);

CREATE TABLE workshop_snapshots (
    snapshot_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chat_id UUID NOT NULL,
    user_id INT NOT NULL,
    message_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (chat_id) REFERENCES workshop_chats(chat_id) ON DELETE CASCADE,
    FOREIGN KEY (message_id) REFERENCES workshop_messages(message_id) ON DELETE CASCADE
);
