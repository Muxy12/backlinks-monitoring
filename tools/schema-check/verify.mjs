// Applies migrations/ into PGlite (real Postgres 17, WASM) and asserts the
// design claims made in docs/03, docs/05, docs/07 and docs/adr/0004.
import { PGlite } from '@electric-sql/pglite'
import { citext } from '@electric-sql/pglite/contrib/citext'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { randomUUID as uuid } from 'node:crypto'

const HERE = path.dirname(fileURLToPath(import.meta.url))
const MIGRATIONS = process.argv[2] ?? path.resolve(HERE, '..', '..', 'migrations')
let pass = 0, fail = 0
const failures = []

function ok(name)        { pass++; console.log(`  PASS  ${name}`) }
function bad(name, why)  { fail++; failures.push(`${name}: ${why}`); console.log(`  FAIL  ${name}\n        ${why}`) }

async function expectReject(db, name, sql, params, mustMatch) {
  try {
    await db.query(sql, params)
    bad(name, 'statement was ACCEPTED but should have been rejected')
  } catch (e) {
    const m = String(e.message)
    if (mustMatch && !m.toLowerCase().includes(mustMatch.toLowerCase())) {
      bad(name, `rejected, but not for the expected reason (wanted ~"${mustMatch}"): ${m}`)
    } else ok(`${name} — rejected: ${m.split('\n')[0].slice(0, 90)}`)
  }
}
async function expectAccept(db, name, sql, params) {
  try { await db.query(sql, params); ok(name) }
  catch (e) { bad(name, `should have been accepted: ${e.message.split('\n')[0]}`) }
}

const db = new PGlite({ extensions: { citext } })
await db.waitReady

// ── 1. Apply every migration in order ───────────────────────────────────────
console.log('\n=== Applying migrations ===')
const files = fs.readdirSync(MIGRATIONS).filter(f => f.endsWith('.sql')).sort()
for (const f of files) {
  try {
    await db.exec(fs.readFileSync(path.join(MIGRATIONS, f), 'utf8'))
    ok(`apply ${f}`)
  } catch (e) {
    bad(`apply ${f}`, e.message)
    console.log('\nMigration failed — cannot continue.')
    process.exit(1)
  }
}

// ── 2. Expected objects exist ───────────────────────────────────────────────
console.log('\n=== Schema objects ===')
const EXPECTED_TABLES = [
  'users','sessions','auth_tokens','organizations','organization_members',
  'organization_invitations','clients','projects','campaigns','publishers',
  'backlinks','backlink_checks','backlink_events','tags','backlink_tags',
  'jobs','imports','import_rows','subscriptions','stripe_events',
  'usage_records','notifications','notification_preferences','audit_logs',
]
const tables = (await db.query(
  `SELECT tablename FROM pg_tables WHERE schemaname='public' ORDER BY 1`
)).rows.map(r => r.tablename)
const missing = EXPECTED_TABLES.filter(t => !tables.includes(t))
missing.length ? bad('all spec §41 tables present', `missing: ${missing.join(', ')}`)
               : ok(`all ${EXPECTED_TABLES.length} spec §41 tables present`)

const enums = (await db.query(
  `SELECT typname FROM pg_type WHERE typtype='e' ORDER BY 1`
)).rows.map(r => r.typname)
ok(`enum types: ${enums.join(', ')}`)

// ── 3. Fixtures: two organizations ──────────────────────────────────────────
const A = { org: uuid(), client: uuid(), project: uuid(), campaign: uuid(), publisher: uuid(), tag: uuid() }
const B = { org: uuid(), client: uuid(), project: uuid(), tag: uuid() }

for (const [o, slug] of [[A, 'org-a'], [B, 'org-b']]) {
  await db.query(`INSERT INTO organizations (id,name,slug) VALUES ($1,$2,$3)`, [o.org, slug, slug])
  await db.query(`INSERT INTO clients (id,organization_id,name) VALUES ($1,$2,'c')`, [o.client, o.org])
  await db.query(`INSERT INTO projects (id,organization_id,client_id,name,domain)
                  VALUES ($1,$2,$3,'p','example.com')`, [o.project, o.org, o.client])
  await db.query(`INSERT INTO tags (id,organization_id,name) VALUES ($1,$2,'t')`, [o.tag, o.org])
}
await db.query(`INSERT INTO campaigns (id,organization_id,project_id,name) VALUES ($1,$2,$3,'camp')`,
  [A.campaign, A.org, A.project])
await db.query(`INSERT INTO publishers (id,organization_id,name,domain) VALUES ($1,$2,'pub','pub.com')`,
  [A.publisher, A.org])

const mkBacklink = (id, org, project, extra = {}) => {
  const cols = { id, organization_id: org, project_id: project,
    source_url: extra.src ?? 'https://pub.com/a', source_url_normalized: extra.src ?? 'https://pub.com/a',
    source_domain: 'pub.com', target_url: extra.tgt ?? 'https://example.com/x',
    target_url_normalized: extra.tgt ?? 'https://example.com/x', target_domain: 'example.com', ...extra.cols }
  const keys = Object.keys(cols)
  return [`INSERT INTO backlinks (${keys.join(',')}) VALUES (${keys.map((_, i) => `$${i + 1}`).join(',')})`,
          keys.map(k => cols[k])]
}

// ── 4. Tenant isolation — ADR-0004 layer 3 (the claim that does not depend
//       on our Rust code being correct) ───────────────────────────────────────
console.log('\n=== Tenant isolation: composite FKs (docs/05 layer 3) ===')

await expectAccept(db, 'backlink in org A -> project in org A', ...mkBacklink(uuid(), A.org, A.project))

await expectReject(db, 'backlink in org A -> project in org B BLOCKED',
  ...mkBacklink(uuid(), A.org, B.project), 'backlinks_project_fk')

await expectReject(db, 'backlink in org A -> campaign in org B BLOCKED',
  ...mkBacklink(uuid(), A.org, A.project, { src: 'https://pub.com/z', cols: { campaign_id: uuid() } }),
  'backlinks_campaign_fk')

{
  const bl = uuid()
  await db.query(...mkBacklink(bl, A.org, A.project, { src: 'https://pub.com/tagtest' }))
  await expectAccept(db, 'tag from org A on backlink in org A',
    `INSERT INTO backlink_tags (organization_id,backlink_id,tag_id) VALUES ($1,$2,$3)`, [A.org, bl, A.tag])
  await expectReject(db, 'tag from org B on backlink in org A BLOCKED',
    `INSERT INTO backlink_tags (organization_id,backlink_id,tag_id) VALUES ($1,$2,$3)`, [A.org, bl, B.tag],
    'backlink_tags_tag_fk')
}

await expectReject(db, 'project in org A -> client in org B BLOCKED',
  `INSERT INTO projects (id,organization_id,client_id,name,domain) VALUES ($1,$2,$3,'p2','e.com')`,
  [uuid(), A.org, B.client], 'projects_client_fk')

// ── 5. CHECK constraints that encode business rules ─────────────────────────
console.log('\n=== Business-rule CHECK constraints ===')

await expectReject(db, 'cost without currency BLOCKED (docs/03 §1)',
  ...mkBacklink(uuid(), A.org, A.project, { src: 'https://pub.com/c1', cols: { cost: '100.00' } }),
  'backlinks_cost_currency')

await expectAccept(db, 'cost with currency accepted',
  ...mkBacklink(uuid(), A.org, A.project, { src: 'https://pub.com/c2', cols: { cost: '100.00', currency: 'EUR' } }))

await expectReject(db, 'lowercase currency BLOCKED',
  ...mkBacklink(uuid(), A.org, A.project, { src: 'https://pub.com/c3', cols: { cost: '1.00', currency: 'eur' } }),
  'currency_format')

await expectReject(db, "warranty_type='months' without value BLOCKED (§32)",
  ...mkBacklink(uuid(), A.org, A.project, { src: 'https://pub.com/w1', cols: { warranty_type: 'months' } }),
  'warranty_value')

await expectReject(db, "warranty_type='days' with value 0 BLOCKED",
  ...mkBacklink(uuid(), A.org, A.project,
    { src: 'https://pub.com/w2', cols: { warranty_type: 'days', warranty_value: 0 } }), 'warranty_value')

await expectAccept(db, "warranty_type='months' WITH a value accepted",
  ...mkBacklink(uuid(), A.org, A.project,
    { src: 'https://pub.com/w3', cols: { warranty_type: 'months', warranty_value: 12 } }))

await expectAccept(db, "warranty_type='lifetime' needs no value (§32)",
  ...mkBacklink(uuid(), A.org, A.project,
    { src: 'https://pub.com/w4', cols: { warranty_type: 'lifetime' } }))

await expectReject(db, 'archived backlink cannot stay scheduled (docs/08 §7)',
  ...mkBacklink(uuid(), A.org, A.project, { src: 'https://pub.com/a1',
    cols: { archived_at: new Date().toISOString(), next_check_at: new Date().toISOString() } }),
  'archived_not_scheduled')

await expectReject(db, 'HTTP status 999 BLOCKED',
  ...mkBacklink(uuid(), A.org, A.project, { src: 'https://pub.com/h1', cols: { source_http_status: 999 } }),
  'http_status_range')

await expectReject(db, 'check frequency of 1 minute BLOCKED (crawler politeness, §28)',
  `INSERT INTO projects (id,organization_id,name,domain,default_check_frequency)
   VALUES ($1,$2,'fast','e.com', interval '1 minute')`, [uuid(), A.org], 'frequency_sane')

await expectReject(db, "invitation with role='owner' BLOCKED",
  `INSERT INTO organization_invitations (id,organization_id,email,role,token_hash,expires_at)
   VALUES ($1,$2,'x@y.com','owner',$3, now()+interval '1 day')`,
  [uuid(), A.org, Buffer.alloc(32)], 'org_invites_role')

{
  const u = uuid()
  await db.query(`INSERT INTO users (id,email,password_hash,name) VALUES ($1,'u@x.com','h','n')`, [u])
  await expectReject(db, 'session token_hash must be 32 bytes (SHA-256)',
    `INSERT INTO sessions (id,user_id,token_hash,csrf_token_hash,expires_at)
     VALUES ($1,$2,$3,$4, now()+interval '1 day')`,
    [uuid(), u, Buffer.alloc(8), Buffer.alloc(32)], 'token_hash_len')
}

// ── 6. Uniqueness / idempotency indexes (docs/07 §4) ────────────────────────
console.log('\n=== Idempotency & dedup indexes ===')

{
  const dup = { src: 'https://pub.com/dup', tgt: 'https://example.com/dup' }
  await expectAccept(db, 'first backlink (project, source, target)',
    ...mkBacklink(uuid(), A.org, A.project, dup))
  await expectReject(db, 'duplicate backlink in same project BLOCKED (§38)',
    ...mkBacklink(uuid(), A.org, A.project, dup), 'backlinks_dedupe_uq')
  await expectAccept(db, 'same link in a DIFFERENT project allowed',
    ...mkBacklink(uuid(), B.org, B.project, dup))
}

{
  const key = 'check_now:' + uuid()
  await expectAccept(db, 'first job with idempotency_key',
    `INSERT INTO jobs (id,job_type,idempotency_key) VALUES ($1,'check_backlink_now',$2)`, [uuid(), key])
  await expectReject(db, '"Check now" pressed twice enqueues once (§36)',
    `INSERT INTO jobs (id,job_type,idempotency_key) VALUES ($1,'check_backlink_now',$2)`, [uuid(), key],
    'jobs_idempotency_uq')
  await db.query(`UPDATE jobs SET status='completed', completed_at=now() WHERE idempotency_key=$1`, [key])
  await expectAccept(db, 'same key allowed again once the first job completed',
    `INSERT INTO jobs (id,job_type,idempotency_key) VALUES ($1,'check_backlink_now',$2)`, [uuid(), key])
}

await expectReject(db, "job 'processing' without a lease BLOCKED (reaper invariant)",
  `INSERT INTO jobs (id,job_type,status) VALUES ($1,'x','processing')`, [uuid()], 'processing_has_lock')

{
  const e = 'evt_' + uuid()
  await expectAccept(db, 'first Stripe webhook event',
    `INSERT INTO stripe_events (event_id,event_type) VALUES ($1,'invoice.paid')`, [e])
  await expectReject(db, 'replayed Stripe webhook BLOCKED (§39, S5)',
    `INSERT INTO stripe_events (event_id,event_type) VALUES ($1,'invoice.paid')`, [e], 'stripe_events_pkey')
}

{
  const bl = uuid(), crawl = uuid()
  await db.query(...mkBacklink(bl, A.org, A.project, { src: 'https://pub.com/chk' }))
  const ins = `INSERT INTO backlink_checks (id,backlink_id,organization_id,crawl_id,found,status_after)
               VALUES ($1,$2,$3,$4,true,'live')`
  await expectAccept(db, 'first check row for (backlink, crawl)', ins, [uuid(), bl, A.org, crawl])
  await expectReject(db, 're-run of same crawl writes no second check row (docs/07 §4)',
    ins, [uuid(), bl, A.org, crawl], 'backlink_checks_idempotency_uq')
}

{
  await expectAccept(db, 'first pending invitation',
    `INSERT INTO organization_invitations (id,organization_id,email,role,token_hash,expires_at)
     VALUES ($1,$2,'dup@x.com','member',$3, now()+interval '1 day')`, [uuid(), A.org, Buffer.alloc(32, 1)])
  await expectReject(db, 'second pending invitation to same email BLOCKED',
    `INSERT INTO organization_invitations (id,organization_id,email,role,token_hash,expires_at)
     VALUES ($1,$2,'dup@x.com','member',$3, now()+interval '1 day')`, [uuid(), A.org, Buffer.alloc(32, 2)],
    'org_invites_pending_uq')
}

// ── 7. updated_at trigger ───────────────────────────────────────────────────
console.log('\n=== Triggers ===')
{
  const before = (await db.query(`SELECT updated_at FROM organizations WHERE id=$1`, [A.org])).rows[0].updated_at
  await new Promise(r => setTimeout(r, 25))
  await db.query(`UPDATE organizations SET name='renamed' WHERE id=$1`, [A.org])
  const after = (await db.query(`SELECT updated_at FROM organizations WHERE id=$1`, [A.org])).rows[0].updated_at
  new Date(after) > new Date(before) ? ok('set_updated_at trigger fires on UPDATE')
                                     : bad('set_updated_at trigger', `not advanced (${before} -> ${after})`)
}

// ── 8. The scheduler index — docs/03 §4.1, the most important index ─────────
console.log('\n=== Scheduler query plan (docs/03 §4.1, §11) ===')
{
  await db.exec(`
    INSERT INTO backlinks (id,organization_id,project_id,source_url,source_url_normalized,
                           source_domain,target_url,target_url_normalized,target_domain,
                           next_check_at,status)
    SELECT gen_random_uuid(), '${A.org}', '${A.project}',
           'https://pub.com/p'||g, 'https://pub.com/p'||g, 'pub.com',
           'https://example.com/t'||g, 'https://example.com/t'||g, 'example.com',
           now() + (g || ' seconds')::interval - interval '1000 seconds', 'live'
    FROM generate_series(1, 3000) g;
    ANALYZE backlinks;`)

  const n = (await db.query(`SELECT count(*)::int AS c FROM backlinks`)).rows[0].c
  ok(`seeded backlinks table: ${n} rows`)

  const plan = (await db.query(`
    EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
    SELECT id, source_url_normalized FROM backlinks
    WHERE archived_at IS NULL AND next_check_at IS NOT NULL AND next_check_at <= now()
    ORDER BY next_check_at LIMIT 500`)).rows.map(r => r['QUERY PLAN']).join('\n')

  console.log(plan.split('\n').map(l => '        ' + l).join('\n'))
  plan.includes('backlinks_due_idx')
    ? ok('scheduler query uses backlinks_due_idx')
    : bad('scheduler query uses backlinks_due_idx', 'index not chosen — see plan above')
  plan.includes('Seq Scan on backlinks')
    ? bad('scheduler query avoids a sequential scan (§11)', 'planner chose Seq Scan')
    : ok('scheduler query avoids a sequential scan (§11)')
}

// ── 9. The claim query actually runs (docs/07 §2) ───────────────────────────
console.log('\n=== Job claim query (docs/07 §2) ===')
{
  await db.exec(`INSERT INTO jobs (id,job_type,priority)
                 SELECT gen_random_uuid(),'check_source_page', (g % 3)
                 FROM generate_series(1,50) g;`)
  const claimed = await db.query(`
    UPDATE jobs SET status='processing', locked_at=now(), locked_by=$1,
                    attempts=attempts+1, updated_at=now()
    WHERE id IN (
      SELECT id FROM jobs WHERE status='pending' AND scheduled_at <= now()
      ORDER BY priority DESC, scheduled_at
      FOR UPDATE SKIP LOCKED LIMIT $2
    ) RETURNING id, priority`, ['worker-1', 25])
  claimed.rows.length === 25
    ? ok(`FOR UPDATE SKIP LOCKED claim returned ${claimed.rows.length} jobs`)
    : bad('claim query', `expected 25 rows, got ${claimed.rows.length}`)
  // RETURNING does not preserve the inner SELECT's ORDER BY, so we assert on
  // the CLAIMED SET, not on row order: the query must lock the highest-priority
  // rows. No claimed job may have a lower priority than any job left pending.
  // (50 jobs, priorities g%3 -> {0,1,2}; claiming 25 must take every 2 and 1,
  //  and never a 0.)
  const claimedPrios = claimed.rows.map(r => r.priority)
  const maxPending = (await db.query(
    `SELECT coalesce(max(priority), -1) AS m FROM jobs
     WHERE status='pending' AND job_type='check_source_page'`)).rows[0].m
  const minClaimed = Math.min(...claimedPrios)
  minClaimed >= maxPending
    ? ok(`claim locks highest-priority rows first (min claimed ${minClaimed} >= max still-pending ${maxPending})`)
    : bad('claim respects priority', `claimed a priority ${minClaimed} while a ${maxPending} was left pending`)
  claimedPrios.includes(0)
    ? bad('claim skips lowest band when higher work exists', 'a priority-0 job was claimed')
    : ok('claim left all priority-0 jobs pending while higher work existed')

  const plan = (await db.query(`
    EXPLAIN SELECT id FROM jobs WHERE status='pending' AND scheduled_at <= now()
    ORDER BY priority DESC, scheduled_at LIMIT 10`)).rows.map(r => r['QUERY PLAN']).join('\n')
  plan.includes('jobs_claim_idx') ? ok('claim query uses jobs_claim_idx')
                                  : bad('claim query uses jobs_claim_idx', plan.split('\n')[0])
}

// ── 10. Value at Risk aggregate (§33, docs/03 §4.3) ─────────────────────────
console.log('\n=== Value at Risk (§33) ===')
{
  await db.exec(`
    UPDATE backlinks SET status='lost', cost=100.00, currency='EUR'
    WHERE organization_id='${A.org}' AND id IN (SELECT id FROM backlinks WHERE organization_id='${A.org}' LIMIT 5);
    UPDATE backlinks SET status='changed', cost=50.00, currency='USD'
    WHERE organization_id='${A.org}' AND status='live'
      AND id IN (SELECT id FROM backlinks WHERE organization_id='${A.org}' AND status='live' LIMIT 3);`)
  const var_ = await db.query(`
    SELECT currency, coalesce(sum(cost),0)::text AS at_risk FROM backlinks
    WHERE organization_id=$1 AND archived_at IS NULL
      AND status IN ('lost','changed','nofollow','source_error','target_error','redirected')
    GROUP BY currency ORDER BY currency`, [A.org])
  const shown = var_.rows.map(r => `${r.at_risk} ${r.currency}`).join(' + ')
  var_.rows.length > 1
    ? ok(`Value at Risk is grouped by currency, not summed across: ${shown}`)
    : bad('Value at Risk grouped by currency', `expected >1 currency group, got: ${shown}`)
}

// ── Summary ─────────────────────────────────────────────────────────────────
console.log(`\n${'='.repeat(64)}`)
console.log(`  ${pass} passed, ${fail} failed`)
if (fail) { console.log('\nFailures:'); failures.forEach(f => console.log('  - ' + f)) }
console.log('='.repeat(64))
await db.close()
process.exit(fail ? 1 : 0)
