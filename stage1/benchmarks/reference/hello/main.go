package main

import "fmt"

func banner(name string) string { return "hello " + name }
func lucky(base int) int        { return base + 2 }
func isReady(value int) bool    { return value == 42 }

func main() {
	answer := lucky(40)
	ready := isReady(answer)
	if ready {
		fmt.Println(banner("from stage1"))
	} else {
		fmt.Println("stage1 failed")
	}
	fmt.Println(answer)
	fmt.Println(ready)
}
