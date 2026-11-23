"""Servidor de contrato local com autoridades e chaves efêmeras para o teste Rust."""
import http.server
import json
import os
from pathlib import Path
import ssl
import subprocess
import sys

os.chdir(sys.argv[1])
os.umask(0o077)

def openssl(*args):
    subprocess.run(["openssl", *args], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

openssl("req", "-x509", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:P-256", "-nodes",
        "-keyout", "ca.key", "-out", "ca.pem", "-subj", "/CN=CA de teste", "-days", "1")
for name, usage in [("server", "serverAuth"), ("client", "clientAuth")]:
    openssl("req", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:P-256", "-nodes",
            "-keyout", name+".key", "-out", name+".csr", "-subj", "/CN="+name)
    Path("extensions").write_text("subjectAltName=IP:127.0.0.1\nextendedKeyUsage="+usage+"\n")
    openssl("x509", "-req", "-in", name+".csr", "-CA", "ca.pem", "-CAkey", "ca.key",
            "-CAcreateserial", "-days", "1", "-extfile", "extensions", "-out", name+".pem")

class Handler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        if self.path != "/oauth/v2/token":
            return self.send_error(404)
        self.rfile.read(int(self.headers["Content-Length"]))
        self.reply(json.dumps(dict(access_token="fixture", token_type="Bearer", expires_in=3600)).encode())

    def do_GET(self):
        if self.path != "/banking/v2/saldo" or self.headers.get("Authorization") != "Bearer fixture":
            return self.send_error(403)
        self.reply(b'{"disponivel":10.25}')

    def reply(self, data):
        self.send_response(200)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, *args):
        pass

server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
context.verify_mode = ssl.CERT_REQUIRED
context.load_verify_locations("ca.pem")
context.load_cert_chain("server.pem", "server.key")
server.socket = context.wrap_socket(server.socket, server_side=True)
print(server.server_port, flush=True)
server.serve_forever()
