package main

import "fmt"

func compute(value int, ch chan<- int) {
	ch <- value * 7
}

func main() {
	ch := make(chan int, 1)
	go compute(6, ch)
	fmt.Println(<-ch)
}
