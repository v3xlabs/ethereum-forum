-- Normalize llm_memory.sources from legacy string arrays ("magicians/1234",
-- "25368", "") to structured link objects: {"url": "/t/magicians/1234", "reason": null}.
UPDATE llm_memory
SET sources = (
    SELECT COALESCE(
        jsonb_agg(
            CASE
                WHEN jsonb_typeof(elem) = 'object' THEN elem
                ELSE jsonb_build_object(
                    'url',
                    CASE
                        WHEN elem #>> '{}' ~ '^[a-z]+/[0-9]+$' THEN '/t/' || (elem #>> '{}')
                        ELSE elem #>> '{}'
                    END,
                    'reason',
                    NULL
                )
            END
        ) FILTER (
            WHERE jsonb_typeof(elem) = 'object'
               OR COALESCE(elem #>> '{}', '') <> ''
        ),
        '[]'::jsonb
    )
    FROM jsonb_array_elements(COALESCE(sources, '[]'::jsonb)) AS elem
)
WHERE sources IS NOT NULL AND jsonb_typeof(sources) = 'array';
