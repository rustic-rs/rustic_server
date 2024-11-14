# No auth
HEAD http://127.0.0.1:8000/ci_repo/config
HTTP 403

# Login
HEAD http://127.0.0.1:8000/ci_repo/config
[BasicAuth]
restic: restic
HTTP 200

# Access to keys
GET http://127.0.0.1:8000/ci_repo/keys
[BasicAuth]
restic: restic
HTTP 200
Content-Type: application/vnd.x.restic.rest.v1

GET http://127.0.0.1:8000/ci_repo/keys/92c489eaca950f3a9d860da1bef0af845dd8c74a9a34c860bd8e6a54bd2326d2
RANGE: bytes=0-230
[BasicAuth]
restic: restic
HTTP 206

# GET http://127.0.0.1:8000/ci_repo/keys/92c489eaca950f3a9d860da1bef0af845dd8c74a9a34c860bd8e6a54bd2326d2
# [BasicAuth]
# restic: restic
# HTTP 200