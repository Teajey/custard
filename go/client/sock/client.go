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

type GetRequest struct {
	Name      string `msgpack:"name"`
	SortKey   string `msgpack:"sort_key,omitempty"`
	OrderDesc bool   `msgpack:"order_desc,omitempty"`
}

type QueryRequest struct {
	Name      string         `msgpack:"name"`
	Query     map[string]any `msgpack:"query"`
	SortKey   string         `msgpack:"sort_key,omitempty"`
	OrderDesc bool           `msgpack:"order_desc,omitempty"`
	Intersect bool           `msgpack:"intersect"`
}

type getResponse struct {
	File         any    `msgpack:"file"`
	PrevFileName string `msgpack:"prev_file_name"`
	NextFileName string `msgpack:"next_file_name"`
}

func SendSingleGetRequest(socketPath string, getReq GetRequest) (*getResponse, error) {
	conn, err := net.Dial("unix", socketPath)
	if err != nil {
		return nil, fmt.Errorf("Failed to dial: %w", err)
	}
	defer conn.Close()

	req := taggedRequest{
		Tag:   "SingleGet",
		Value: getReq,
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
		var getResp getResponse
		err := msgpack.Unmarshal(resp.Value, &getResp)
		if err != nil {
			return nil, fmt.Errorf("Could not unmarshal response value: %w", err)
		}
		return &getResp, nil
	case "InternalServerError":
		return nil, fmt.Errorf("Custard had internal server error")
	default:
		return nil, fmt.Errorf("Unrecognised tag from server: %s", resp.Tag)
	}
}

func SendSingleQueryRequest(socketPath string, queryReq QueryRequest) (*getResponse, error) {
	conn, err := net.Dial("unix", socketPath)
	if err != nil {
		return nil, fmt.Errorf("Failed to dial: %w", err)
	}
	defer conn.Close()

	req := taggedRequest{
		Tag:   "SingleQuery",
		Value: queryReq,
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
		var getResp getResponse
		err := msgpack.Unmarshal(resp.Value, &getResp)
		if err != nil {
			return nil, fmt.Errorf("Could not unmarshal response value: %w", err)
		}
		return &getResp, nil
	case "InternalServerError":
		return nil, fmt.Errorf("Custard had internal server error")
	default:
		return nil, fmt.Errorf("Unrecognised tag from server: %s", resp.Tag)
	}
}
