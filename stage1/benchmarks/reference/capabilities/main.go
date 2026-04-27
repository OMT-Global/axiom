package main

import (
	"crypto/sha256"
	"fmt"
	"net"
	"os"
	"time"
)

func main() {
	if value, err := os.ReadFile("stage1/examples/capabilities/src/fixture.txt"); err == nil {
		fmt.Println(string(value))
	} else {
		fmt.Println("missing")
	}

	if _, err := net.LookupHost("localhost"); err == nil {
		fmt.Println(true)
	} else {
		fmt.Println(false)
	}

	fmt.Printf("%x\n", sha256.Sum256([]byte("abc")))
	fmt.Println(time.Now().UnixMilli() > 0)

	if value, ok := os.LookupEnv("__AXIOM_STAGE1_MISSING__"); ok {
		fmt.Println(value)
	} else {
		fmt.Println("none")
	}
}
