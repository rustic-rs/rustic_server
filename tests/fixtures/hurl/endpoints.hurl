# No auth
HEAD http://127.0.0.1:8000/ci_repo/config
HTTP 403

# Access a new repository
HEAD http://127.0.0.1:8000/ci_repo/
[BasicAuth]
hurl: hurl
HTTP 405

# Create a new repository
POST http://127.0.0.1:8000/ci_repo/?create=true
[BasicAuth]
hurl: hurl
HTTP 200


HEAD http://127.0.0.1:8000/ci_repo/config
[BasicAuth]
hurl: hurl
HTTP 200

# Access to keys
GET http://127.0.0.1:8000/ci_repo/keys/
[BasicAuth]
hurl: hurl
HTTP 200
Content-Type: application/vnd.x.restic.rest.v1

GET http://127.0.0.1:8000/ci_repo/keys/eb7e523a1916c2cc1c750dc89cd6024f5dd319814c417a3f9081578f8c2c4a76
RANGE: bytes=0-230
[BasicAuth]
hurl: hurl
HTTP 206
content-length: 231

# GET http://127.0.0.1:8000/ci_repo/keys/eb7e523a1916c2cc1c750dc89cd6024f5dd319814c417a3f9081578f8c2c4a76
# [BasicAuth]
# hurl: hurl
# HTTP 200

GET http://127.0.0.1:8000/ci_repo/config
[BasicAuth]
hurl: hurl
HTTP 200

GET http://127.0.0.1:8000/ci_repo/locks/
[BasicAuth]
hurl: hurl
HTTP 200

# POST http://127.0.0.1:8000/ci_repo/locks/ac4ff62472b009cf71c81199f4fc635152639909cb1143911150db467ca86544
# [BasicAuth]
# hurl: hurl
# HTTP 200

GET http://127.0.0.1:8000/ci_repo/locks/
[BasicAuth]
hurl: hurl
HTTP 200

GET http://127.0.0.1:8000/ci_repo/snapshots/
[BasicAuth]
hurl: hurl
HTTP 200

GET http://127.0.0.1:8000/ci_repo/index/
[BasicAuth]
hurl: hurl
HTTP 200

# POST http://127.0.0.1:8000/ci_repo/data/3b013253cd72fa7e98f9dcd6106f9565933556f1c80a720e1e44dbf3b57af446
# [BasicAuth]
# hurl: hurl
# HTTP 200

# POST http://127.0.0.1:8000/ci_repo/data/89923722810777f3026a7cf9b246eb9613c7e3f64e77e6ccecb4001774c38acf
# [BasicAuth]
# hurl: hurl
# HTTP 200

# POST http://127.0.0.1:8000/ci_repo/index/004ebe81e7927131b6dde40bd4595ebf95a355bf26509de83fb9d12b4ab280b4
# [BasicAuth]
# hurl: hurl
# HTTP 200

# POST http://127.0.0.1:8000/ci_repo/snapshots/ddd75013f2d2470d910adadf728c5c8cd6e91cb591bdfbf1cfd7f3af7e32c7eb
# [BasicAuth]
# hurl: hurl
# HTTP 200

# DELETE http://127.0.0.1:8000/ci_repo/locks/ac4ff62472b009cf71c81199f4fc635152639909cb1143911150db467ca86544
# [BasicAuth]
# hurl: hurl
# HTTP 200
