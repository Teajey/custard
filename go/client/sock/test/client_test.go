package sock_test

import (
	"testing"

	"github.com/Teajey/custard/go/client/sock"
)

func TestSendSingleGetRequest(t *testing.T) {
	client := sock.NewClient("/tmp/custard")
	resp, err := client.SendSingleGetRequest(sock.GetRequest{
		Name:      "chai-cheese.md",
		SortKey:   "",
		OrderDesc: false,
	})
	if err != nil {
		t.Fatalf("Request failed: %s", err)
	}
	if resp == nil {
		t.Fatalf("Didn't find file")
	}
	expectedPrevFileName := "chapter-1-tokyo.md"
	if resp.PrevFileName != expectedPrevFileName {
		t.Fail()
		t.Logf("Prev file name not '%s'. Found: %s", expectedPrevFileName, resp.PrevFileName)
	}
	expectedNextFileName := "canned-cake-canned-cake.md"
	if resp.NextFileName != expectedNextFileName {
		t.Fail()
		t.Logf("Next file name not '%s'. Found: %s", expectedNextFileName, resp.NextFileName)
	}
}

func TestSendSingleQueryRequest(t *testing.T) {
	client := sock.NewClient("/tmp/custard")
	resp, err := client.SendSingleQueryRequest(sock.QueryRequest{
		Name: "chai-cheese.md",
		Query: map[string]any{
			"tags": []string{"code"},
		},
		SortKey:   "",
		OrderDesc: false,
		Intersect: false,
	})
	if err != nil {
		t.Fatalf("Request failed: %s", err)
	}
	if resp == nil {
		t.Fatalf("Didn't find file")
	}
	expectedPrevFileName := "2024-01-14.md"
	if resp.PrevFileName != expectedPrevFileName {
		t.Fail()
		t.Logf("Prev file name not '%s'. Found: %s", expectedPrevFileName, resp.PrevFileName)
	}
	expectedNextFileName := "aoc23day4.md"
	if resp.NextFileName != expectedNextFileName {
		t.Fail()
		t.Logf("Next file name not '%s'. Found: %s", expectedNextFileName, resp.NextFileName)
	}
}
