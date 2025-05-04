package sock

import (
	"fmt"
	"net"

	"github.com/vmihailenco/msgpack/v5"
)

type taggedRequest struct {
	Tag   string `msgpack:"tag"`
	Value any    `msgpack:"value"`
}

type taggedResponse struct {
	Tag   string             `msgpack:"tag"`
	Value msgpack.RawMessage `msgpack:"value"`
}

type RequestSingleGet struct {
	Name      string `msgpack:"name"`
	SortKey   string `msgpack:"sort_key,omitempty"`
	OrderDesc bool   `msgpack:"order_desc,omitempty"`
}

type RequestListGet struct {
	SortKey   string `msgpack:"sort_key,omitempty"`
	OrderDesc bool   `msgpack:"order_desc,omitempty"`
	Offset    uint   `msgpack:"offset,omitempty"`
	Limit     uint   `msgpack:"limit,omitempty"`
}

type RequestSingleQuery struct {
	Name      string         `msgpack:"name"`
	Query     map[string]any `msgpack:"query"`
	SortKey   string         `msgpack:"sort_key,omitempty"`
	OrderDesc bool           `msgpack:"order_desc,omitempty"`
	Intersect bool           `msgpack:"intersect"`
}

type singleResponse struct {
	File         any    `msgpack:"file"`
	PrevFileName string `msgpack:"prev_file_name"`
	NextFileName string `msgpack:"next_file_name"`
}

type listResponse struct {
	Name        string          `msgpack:"name"`
	Frontmatter *map[string]any `msgpack:"frontmatter,omitempty"`
	OneLiner    string          `msgpack:"one_liner,omitempty"`
	Modified    string          `msgpack:"modified"`
	Created     string          `msgpack:"created"`
}

type Client struct {
	socketPath string
}

func NewClient(socketPath string) *Client {
	c := Client{
		socketPath,
	}
	return &c
}

func (c *Client) sendListRequest(listReq any, tag string) ([]listResponse, error) {
	conn, err := net.Dial("unix", c.socketPath)
	if err != nil {
		return nil, fmt.Errorf("Failed to dial: %w", err)
	}
	defer conn.Close()

	req := taggedRequest{
		Tag:   tag,
		Value: listReq,
	}

	enc := msgpack.NewEncoder(conn)
	if err := enc.Encode(req); err != nil {
		return nil, fmt.Errorf("Failed to encode request: %w", err)
	}

	var resp *taggedResponse
	dec := msgpack.NewDecoder(conn)
	if err := dec.Decode(&resp); err != nil {
		return nil, fmt.Errorf("Failed to decode response: %w", err)
	}

	switch resp.Tag {
	case "Ok":
		var listResp []listResponse
		err := msgpack.Unmarshal(resp.Value, &listResp)
		if err != nil {
			return nil, fmt.Errorf("Could not unmarshal response value: %w", err)
		}
		return listResp, nil
	case "InternalServerError":
		return nil, fmt.Errorf("Custard had internal server error")
	default:
		return nil, fmt.Errorf("Unrecognised tag from server: %s", resp.Tag)
	}
}

func (c *Client) sendSingleRequest(singleReq any, tag string) (*singleResponse, error) {
	conn, err := net.Dial("unix", c.socketPath)
	if err != nil {
		return nil, fmt.Errorf("Failed to dial: %w", err)
	}
	defer conn.Close()

	req := taggedRequest{
		Tag:   tag,
		Value: singleReq,
	}

	enc := msgpack.NewEncoder(conn)
	if err := enc.Encode(req); err != nil {
		return nil, fmt.Errorf("Failed to encode request: %w", err)
	}

	var resp *taggedResponse
	dec := msgpack.NewDecoder(conn)
	if err := dec.Decode(&resp); err != nil {
		return nil, fmt.Errorf("Failed to decode response: %w", err)
	}

	switch resp.Tag {
	case "Ok":
		if resp.Value == nil {
			return nil, nil
		}
		var singleResp singleResponse
		err := msgpack.Unmarshal(resp.Value, &singleResp)
		if err != nil {
			return nil, fmt.Errorf("Could not unmarshal response value: %w", err)
		}
		return &singleResp, nil
	case "InternalServerError":
		return nil, fmt.Errorf("Custard had internal server error")
	default:
		return nil, fmt.Errorf("Unrecognised tag from server: %s", resp.Tag)
	}
}

func (c *Client) SendSingleGetRequest(req RequestSingleGet) (*singleResponse, error) {
	return c.sendSingleRequest(req, "SingleGet")
}

func (c *Client) SendSingleQueryRequest(req RequestSingleQuery) (*singleResponse, error) {
	return c.sendSingleRequest(req, "SingleQuery")
}

func (c *Client) SendListGetRequest(req RequestListGet) ([]listResponse, error) {
	return c.sendListRequest(req, "ListGet")
}
