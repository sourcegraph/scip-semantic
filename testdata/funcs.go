package example

import (
	f "fmt"
)

func Something() {
	f.Println("hello")
}

func Another() {
	Something()
}
