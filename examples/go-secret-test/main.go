package main

import (
    "fmt"
    "log"
    "net/http"
    "os"
)

func main() {
    port := os.Getenv("PORT")
    if port == "" {
        port = "8080"
    }

    http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
        log.Printf("📥 Request: %s %s", r.Method, r.URL.Path)
        
        secret := os.Getenv("API_TOKEN")
        secretDisplay := "NOT_SET"
        if len(secret) > 4 {
             secretDisplay = secret[:4] + "****"
        } else if secret != "" {
             secretDisplay = "****"
        }

        fmt.Fprintf(w, "🐹 Hello from Go!\n")
        fmt.Fprintf(w, "Server: %s\n", os.Getenv("HOSTNAME"))
        fmt.Fprintf(w, "Secret (API_TOKEN): %s\n", secretDisplay)
    })

    http.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
        w.WriteHeader(http.StatusOK)
        w.Write([]byte("OK"))
    })

    log.Printf("🚀 Starting Go server on port %s...", port)
    if err := http.ListenAndServe(":"+port, nil); err != nil {
        log.Fatal(err)
    }
}
