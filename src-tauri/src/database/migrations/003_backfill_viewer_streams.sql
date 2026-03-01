-- Migration 003: Backfill viewer_streams from existing message data
-- Reconstructs viewer stream participation history from messages + sessions + viewer_profiles.
-- Extracts video_id from sessions.stream_url (format: https://...watch?v=VIDEO_ID...)

INSERT OR IGNORE INTO viewer_streams (viewer_profile_id, video_id, first_comment_at, last_comment_at, message_count)
SELECT
    vp.id AS viewer_profile_id,
    CASE
        WHEN INSTR(SUBSTR(s.stream_url, INSTR(s.stream_url, 'watch?v=') + 8), '&') > 0 THEN
            SUBSTR(s.stream_url,
                   INSTR(s.stream_url, 'watch?v=') + 8,
                   INSTR(SUBSTR(s.stream_url, INSTR(s.stream_url, 'watch?v=') + 8), '&') - 1)
        ELSE
            SUBSTR(s.stream_url, INSTR(s.stream_url, 'watch?v=') + 8)
    END AS video_id,
    MIN(m.timestamp) AS first_comment_at,
    MAX(m.timestamp) AS last_comment_at,
    COUNT(*) AS message_count
FROM messages m
JOIN sessions s ON m.session_id = s.id
JOIN viewer_profiles vp
    ON vp.channel_id = m.channel_id
    AND vp.broadcaster_channel_id = s.broadcaster_channel_id
WHERE s.stream_url LIKE '%watch?v=%'
  AND s.broadcaster_channel_id IS NOT NULL
  AND m.message_type != 'system'
GROUP BY vp.id, video_id;
