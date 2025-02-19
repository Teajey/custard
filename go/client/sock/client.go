package sock

import (
	"fmt"
	"net"

	"github.com/vmihailenco/msgpack"
)

type request struct {
	Tag   string `msgpack:"tag"`
	Value any    `msgpack:"value"`
}

type GetRequest struct {
	Name      string `msgpack:"name"`
	SortKey   string `msgpack:"sort_key,omitempty"`
	OrderDesc bool   `msgpack:"order_desc,omitempty"`
}

type getResponse struct {
	File         any    `msgpack:"file"`
	PrevFileName string `msgpack:"prev_file_name"`
	NextFileName string `msgpack:"next_file_name"`
}

func SendGetRequest(socketPath string, getReq GetRequest) (*getResponse, error) {
	conn, err := net.Dial("unix", socketPath)
	if err != nil {
		return nil, fmt.Errorf("Failed to dial: %w", err)
	}
	defer conn.Close()

	req := request{
		Tag:   "Get",
		Value: getReq,
	}

	enc := msgpack.NewEncoder(conn)
	if err := enc.Encode(req); err != nil {
		return nil, fmt.Errorf("Failed to encode request: %w", err)
	}

	var resp *getResponse
	dec := msgpack.NewDecoder(conn)
	if err := dec.Decode(&resp); err != nil {
		return nil, fmt.Errorf("Failed to decode response: %w", err)
	}

	return resp, nil
}
