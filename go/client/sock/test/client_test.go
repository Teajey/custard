package sock_test

import (
	"slices"
	"testing"

	"github.com/Teajey/custard/go/client/sock"
)

func TestGetSingle(t *testing.T) {
	client := sock.NewClient("/tmp/custard")
	resp, err := client.GetSingle(sock.GetSingleRequest{
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

func TestGetSingleWithNoFrontmatter(t *testing.T) {
	client := sock.NewClient("/tmp/custard")
	resp, err := client.GetSingle(sock.GetSingleRequest{
		Name: "about.md",
	})
	if err != nil {
		t.Fatalf("Request failed: %s", err)
	}
	if resp == nil {
		t.Fatalf("Didn't find file")
	}
	if resp.File.Frontmatter != nil {
		t.Fatalf("Frontmatter was not nil as expected")
	}
}

func TestQuerySingle(t *testing.T) {
	client := sock.NewClient("/tmp/custard")
	resp, err := client.QuerySingle(sock.QuerySingleRequest{
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

func TestGetList(t *testing.T) {
	client := sock.NewClient("/tmp/custard")
	resp, err := client.GetList(sock.GetListRequest{
		Limit: 3,
	})
	if err != nil {
		t.Fatalf("Request failed: %s", err)
	}
	respLen := len(resp.Files)
	if respLen != 3 {
		t.Fatalf("Unexpected response length: %d", respLen)
	}
	if resp.Total != 58 {
		t.Fatalf("Unexpected response total: %d", resp.Total)
	}
}

func TestQueryList(t *testing.T) {
	client := sock.NewClient("/tmp/custard")
	resp, err := client.QueryList(sock.QueryListRequest{
		Query: map[string]any{"tags": []string{"code"}},
	})
	if err != nil {
		t.Fatalf("Request failed: %s", err)
	}
	respLen := len(resp.Files)
	if respLen != 5 {
		t.Fatalf("Unexpected response length: %d", respLen)
	}
	if resp.Total != 5 {
		t.Fatalf("Unexpected response total: %d", resp.Total)
	}
}

func TestGetCollate(t *testing.T) {
	client := sock.NewClient("/tmp/custard")
	resp, err := client.GetCollate(sock.GetCollateRequest{
		Key: "tags",
	})
	if err != nil {
		t.Fatalf("Request failed: %s", err)
	}
	expected := []string{"code", "music", "ramble", "sketch", "travel"}
	if !slices.Equal(expected, resp) {
		t.Fatalf("Expected %v but got %v", expected, resp)
	}
}

func TestQueryCollate(t *testing.T) {
	client := sock.NewClient("/tmp/custard")
	resp, err := client.QueryCollate(sock.QueryCollateRequest{
		Key:   "tags",
		Query: map[string]any{"tags": []string{"code"}},
	})
	if err != nil {
		t.Fatalf("Request failed: %s", err)
	}
	expected := []string{"code", "sketch"}
	if !slices.Equal(expected, resp) {
		t.Fatalf("Expected %v but got %v", expected, resp)
	}
}
