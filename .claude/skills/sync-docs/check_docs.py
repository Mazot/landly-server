#!/usr/bin/env python3
"""Mechanical code <-> docs consistency check for landly-server.

The code is the source of truth. Prints one section per check and exits 1 if
any check found a mismatch. Read-only: it never edits anything.

Usage: python3 .claude/skills/sync-docs/check_docs.py [repo_root]
"""

import re
import sys
from pathlib import Path

ROOT = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path(__file__).resolve().parents[3]
FEATURES = ROOT / "src/app/features"
AUTH_RS = ROOT / "src/app/drivers/middlewares/auth.rs"
README = (ROOT / "README.md").read_text()
AGENTS = (ROOT / "AGENTS.md").read_text()
COPILOT_PATH = ROOT / ".github/copilot-instructions.md"
COPILOT = COPILOT_PATH.read_text() if COPILOT_PATH.exists() else ""

# Mutating routes that are public ON PURPOSE live in one place: the
# INTENTIONALLY_PUBLIC_MUTATING_ROUTES list in auth.rs tests (enforced by
# `cargo test`). Read it from there instead of keeping a second copy.
def _intentionally_public():
    text = AUTH_RS.read_text()
    block = re.search(r"INTENTIONALLY_PUBLIC_MUTATING_ROUTES[^=]*=\s*\[(.*?)\n\s*\];", text, re.S)
    if not block:
        return set()
    return {
        (m, re.sub(r"\{[^}]*\}", "{}", p))
        for m, p in re.findall(r'\(\s*"(\w+)",\s*"(/api[^"]*)"', block.group(1))
    }


INTENTIONALLY_PUBLIC = _intentionally_public()

problems = 0


def section(title):
    print(f"\n== {title}")


def bad(msg):
    global problems
    problems += 1
    print(f"  ✗ {msg}")


def ok(msg):
    print(f"  ✓ {msg}")


def norm(path):
    """Collapse path params so `{id}` and `{organisation_id}` compare equal."""
    return re.sub(r"\{[^}]*\}", "{}", path.rstrip("/") or "/")


def read(path):
    return path.read_text()


# ---------------------------------------------------------------- code facts

def code_routes():
    """{(METHOD, normalized /api path): (feature, raw path)} from config.rs + main.rs."""
    routes = {}
    for cfg in sorted(FEATURES.glob("*/config.rs")):
        text = read(cfg)
        scope = re.search(r'web::scope\(\s*"([^"]+)"', text)
        if not scope:
            continue
        for m in re.finditer(r'\.route\(\s*"([^"]*)"\s*,\s*web::(get|post|put|delete|patch)\(\)', text, re.S):
            raw = "/api" + scope.group(1) + m.group(1)
            routes[(m.group(2).upper(), norm(raw))] = (cfg.parent.name, raw)
    # healthcheck is registered directly in main.rs
    if 'web::scope("/healthcheck")' in read(ROOT / "src/main.rs"):
        routes[("GET", "/api/healthcheck")] = ("healthcheck", "/api/healthcheck")
    return routes


UTOIPA_RE = re.compile(
    r"((?:^//[^\n]*\n)*)#\[utoipa::path\(\s*(get|post|put|delete|patch)\s*,(.*?)\)\]\s*pub async fn (\w+)",
    re.S | re.M,
)


def documented_handlers():
    """[(feature, fn, METHOD, normalized path, authorship comment)] from #[utoipa::path]."""
    out = []
    for ctrl in sorted(FEATURES.glob("*/controllers.rs")):
        text = read(ctrl)
        for m in UTOIPA_RE.finditer(text):
            body = m.group(3)
            path = re.search(r'path\s*=\s*"([^"]+)"', body)
            ctx = re.search(r'context_path\s*=\s*"([^"]+)"', body)
            if not path:
                continue
            full = (ctx.group(1) if ctx else "") + path.group(1)
            out.append((ctrl.parent.name, m.group(4), m.group(2).upper(), norm(full), m.group(1)))
    return out


def apidoc_paths(feature):
    mod = FEATURES / feature / "mod.rs"
    if not mod.exists():
        return set()
    m = re.search(r"paths\((.*?)\)\s*,", read(mod), re.S)
    return set(re.findall(r"controllers::(\w+)", m.group(1))) if m else set()


def auth_required():
    text = read(AUTH_RS)
    block = re.search(r"const AUTH_REQUIRED_ROUTES[^=]*=\s*\[(.*?)\n\];", text, re.S).group(1)
    return re.findall(r'path:\s*"([^"]+)",\s*method:\s*Method::(\w+)', block)


def code_env_vars():
    names = set()
    consts = dict(re.findall(r'pub const (\w+): &str = "(\w+)";', read(ROOT / "src/constants.rs")))
    for rs in (ROOT / "src").rglob("*.rs"):
        text = read(rs)
        # strip test modules so test-only vars are not demanded in the docs table
        text = re.split(r"#\[cfg\(test\)\]", text)[0]
        names.update(re.findall(r'env::var\(\s*"([A-Z0-9_]+)"', text))
        for const in re.findall(r"env::var\(\s*env_key::(\w+)", text):
            names.add(consts.get(const, const))
    return names


def doc_routes(doc):
    """Routes mentioned as `METHOD /api/...` in a markdown document."""
    found = set()
    for m in re.finditer(r"\b(GET|POST|PUT|DELETE|PATCH)\s+(/api/[^\s?`#]+)", doc):
        found.add((m.group(1), norm(m.group(2))))
    return found


# ------------------------------------------------------------------- checks

routes = code_routes()
handlers = documented_handlers()

section("Routes: config.rs vs README.md")
in_readme = doc_routes(README)
missing = sorted(k for k in routes if k not in in_readme)
stale = sorted(k for k in in_readme if k not in routes)
for method, path in missing:
    bad(f"README is missing {method} {routes[(method, path)][1]}")
for method, path in stale:
    bad(f"README documents {method} {path}, which is not registered in any config.rs")
if not missing and not stale:
    ok(f"all {len(routes)} routes documented, none stale")

section("OpenAPI: every route has #[utoipa::path] and is listed in its feature ApiDoc")
by_route = {(m, p): (f, fn) for f, fn, m, p, _ in handlers}
for key, (feature, raw) in sorted(routes.items()):
    if key not in by_route:
        bad(f"{key[0]} {raw}: no #[utoipa::path] with this method+path (check `path =`/`context_path`)")
for feature, fn, method, path, _ in handlers:
    if (method, path) not in routes:
        bad(f"{feature}::{fn} documents {method} {path}, but no such route is registered")
    if fn not in apidoc_paths(feature):
        bad(f"{feature}::{fn} is missing from paths(...) in {feature}/mod.rs ApiDoc")
for feature_dir in sorted(p for p in FEATURES.iterdir() if p.is_dir()):
    name = feature_dir.name
    if f"features::{name}::ApiDoc::openapi()" not in read(ROOT / "src/main.rs"):
        bad(f"{name}::ApiDoc is not merged in build_openapi() (src/main.rs)")
if problems == 0:
    ok("routes, handlers and ApiDocs agree")

section("Auth: AUTH_REQUIRED_ROUTES vs real routes")
before = problems
auth = auth_required()
protected = set()
for pattern, method in auth:
    if pattern.endswith("/"):  # prefix pattern (images style)
        hits = [k for k in routes if k[0] == method and k[1].startswith(norm(pattern) + "/")]
    else:
        hits = [k for k in routes if k == (method, norm(pattern))]
    if not hits:
        bad(f"AUTH_REQUIRED_ROUTES has {method} {pattern}, which matches no registered route")
    protected.update(hits)
for key, (feature, raw) in sorted(routes.items()):
    if key[0] != "GET" and key not in protected and key not in INTENTIONALLY_PUBLIC:
        bad(f"{key[0]} {raw} mutates but is not in AUTH_REQUIRED_ROUTES "
            f"(protect it, or add it to INTENTIONALLY_PUBLIC_MUTATING_ROUTES in auth.rs with a reason)")
if problems == before:
    ok(f"{len(auth)} protected patterns all match real routes; no unexpected public mutating route")

section("Env vars: code vs AGENTS.md")
before = problems
documented = set(re.findall(r"`([A-Z][A-Z0-9_]*\*?)`", AGENTS))
wildcards = [d[:-1] for d in documented if d.endswith("*")]
for var in sorted(code_env_vars()):
    if var not in documented and not any(var.startswith(w) for w in wildcards):
        bad(f"{var} is read by the code but not mentioned in AGENTS.md")
if problems == before:
    ok("every env var the code reads is documented")

section("Feature list: src/app/features vs AGENTS.md / copilot-instructions.md")
before = problems
features = sorted(p.name for p in FEATURES.iterdir() if p.is_dir())
for doc_name, doc in (("AGENTS.md", AGENTS), ("copilot-instructions.md", COPILOT)):
    m = re.search(r"Current features:(.*?)(?:\n\n|\.\n)", doc, re.S)
    if not m:
        bad(f"{doc_name}: no 'Current features:' line")
        continue
    listed = sorted(re.findall(r"`(\w+)`", m.group(1)))
    if listed != features:
        bad(f"{doc_name} lists {listed}, code has {features}")
if problems == before:
    ok(f"{len(features)} features listed consistently")

section("Authorship: AI-generated handlers vs AGENTS.md authorship map")
before = problems
ai_line = next((l for l in AGENTS.splitlines() if l.startswith("- **AI-generated (Claude):**")), "")
for feature, fn, method, path, comment in handlers:
    if "[authorship] AI-generated" not in comment:
        continue
    whole_feature = f"`{feature}`" in ai_line
    # Accept the full path or any tail of >= 2 segments ("/me/notifications"),
    # with path params written either as {id} or left out ("/visit/{id}").
    segs = path.replace("{}", "{id}").split("/")[1:]
    tails = ["/" + "/".join(segs[i:]) for i in range(len(segs) - 1)]
    tails += [re.sub(r"/\{id\}.*", "", t) for t in tails]
    mentioned = any(len(t.split("/")) > 2 and t in ai_line for t in tails)
    if not (whole_feature or mentioned):
        bad(f"{method} {path} ({feature}::{fn}) is AI-generated but not in the AGENTS.md authorship map")
if not ai_line:
    bad("AGENTS.md has no '- **AI-generated (Claude):**' line")
if problems == before:
    ok("every AI-generated handler is covered by the authorship map")

section("Summary")
print(f"  {problems} mismatch(es)" if problems else "  docs are in sync with the code (for what this script can check)")
sys.exit(1 if problems else 0)
