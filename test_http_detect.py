# Test if looks_like_http works
buf = b"GET / HTTP/1.1\r\nHost"

HTTP_METHODS = ["GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS", "PATCH", "CONNECT", "TRACE"]

if len(buf) < 4:
    print("Too short")
else:
    for method in HTTP_METHODS:
        if buf.startswith(method.encode()):
            if len(buf) > len(method) and buf[len(method)] == ord(b' '):
                print(f"Detected: {method}")
                break
    else:
        print("Not HTTP")
