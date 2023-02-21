package example

import "fmt"

func Something() {
	x := true
	fmt.Println(x)
}

func Another() float64 { return 5 / 3 }

type MyThing struct{}

func (m *MyThing) DoSomething()    {}
func (m MyThing) DoSomethingElse() {}
