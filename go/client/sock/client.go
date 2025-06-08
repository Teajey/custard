package sock

import (
	"encoding/binary"
	"fmt"
	"math"
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

type Query struct {
	Map       map[string]any `msgpack:"map"`
	Intersect bool           `msgpack:"intersect"`
}

type SingleRequest struct {
	Name      string `msgpack:"name"`
	Query     *Query `msgpack:"query,omitempty"`
	SortKey   string `msgpack:"sort_key,omitempty"`
	OrderDesc bool   `msgpack:"order_desc,omitempty"`
}

type ListRequest struct {
	Query     *Query `msgpack:"query,omitempty"`
	SortKey   string `msgpack:"sort_key,omitempty"`
	OrderDesc bool   `msgpack:"order_desc,omitempty"`
	Offset    uint   `msgpack:"offset,omitempty"`
	Limit     uint   `msgpack:"limit,omitempty"`
}

type CollateRequest struct {
	Query *Query `msgpack:"query,omitempty"`
	Key   string `msgpack:"key"`
}

type FileResponse struct {
	Name        string         `msgpack:"name"`
	Frontmatter map[string]any `msgpack:"frontmatter,omitempty"`
	Body        string         `msgpack:"body"`
	Modified    string         `msgpack:"modified"`
	Created     string         `msgpack:"created"`
}

type SingleResponse struct {
	File         FileResponse `msgpack:"file"`
	PrevFileName string       `msgpack:"prev_file_name"`
	NextFileName string       `msgpack:"next_file_name"`
}

type ShortResponse struct {
	Name        string         `msgpack:"name"`
	Frontmatter map[string]any `msgpack:"frontmatter,omitempty"`
	OneLiner    string         `msgpack:"one_liner,omitempty"`
	Modified    string         `msgpack:"modified"`
	Created     string         `msgpack:"created"`
}

type ListResponse struct {
	Files []ShortResponse `msgpack:"files"`
	Total uint            `msgpack:"total"`
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

func encodeUint32BufLength(buf []byte) ([]byte, error) {
	bufLength := len(buf)
	if bufLength > math.MaxUint32 {
		return nil, fmt.Errorf("buffer too large: %d (max %d)", bufLength, math.MaxUint32)
	}
	lengthBytes := make([]byte, 4)
	binary.BigEndian.PutUint32(lengthBytes, uint32(len(buf)))
	return lengthBytes, nil
}

func (c *Client) List(req ListRequest) (*ListResponse, error) {
	conn, err := net.Dial("unix", c.socketPath)
	if err != nil {
		return nil, fmt.Errorf("Failed to dial: %w", err)
	}
	defer conn.Close()

	listReq := taggedRequest{
		Tag:   "List",
		Value: req,
	}

	enc := msgpack.NewEncoder(conn)
	enc.UseArrayEncodedStructs(false)
	buf, err := msgpack.Marshal(listReq)
	if err != nil {
		return nil, fmt.Errorf("Failed to encode request: %w", err)
	}
	lengthBytes, err := encodeUint32BufLength(buf)
	if err != nil {
		return nil, fmt.Errorf("Failed to encode request length: %w", err)
	}
	_, err = conn.Write(lengthBytes)
	if err != nil {
		return nil, fmt.Errorf("Failed to send request length: %w", err)
	}
	_, err = conn.Write(buf)
	if err != nil {
		return nil, fmt.Errorf("Failed to send request: %w", err)
	}

	var resp *taggedResponse
	dec := msgpack.NewDecoder(conn)
	if err := dec.Decode(&resp); err != nil {
		return nil, fmt.Errorf("Failed to decode response: %w", err)
	}

	switch resp.Tag {
	case "Ok":
		var listResp ListResponse
		err := msgpack.Unmarshal(resp.Value, &listResp)
		if err != nil {
			return nil, fmt.Errorf("Could not unmarshal response value: %w", err)
		}
		return &listResp, nil
	case "InternalServerError":
		return nil, fmt.Errorf("Custard had internal server error")
	default:
		return nil, fmt.Errorf("Unrecognised tag from server: %s", resp.Tag)
	}
}

func (c *Client) Single(req SingleRequest) (*SingleResponse, error) {
	conn, err := net.Dial("unix", c.socketPath)
	if err != nil {
		return nil, fmt.Errorf("Failed to dial: %w", err)
	}
	defer conn.Close()

	singleReq := taggedRequest{
		Tag:   "Single",
		Value: req,
	}

	enc := msgpack.NewEncoder(conn)
	enc.UseArrayEncodedStructs(false)
	buf, err := msgpack.Marshal(singleReq)
	if err != nil {
		return nil, fmt.Errorf("Failed to encode request: %w", err)
	}
	lengthBytes, err := encodeUint32BufLength(buf)
	if err != nil {
		return nil, fmt.Errorf("Failed to encode request length: %w", err)
	}
	_, err = conn.Write(lengthBytes)
	if err != nil {
		return nil, fmt.Errorf("Failed to send request length: %w", err)
	}
	_, err = conn.Write(buf)
	if err != nil {
		return nil, fmt.Errorf("Failed to send request: %w", err)
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
		var singleResp SingleResponse
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

func (c *Client) Collate(req CollateRequest) ([]string, error) {
	conn, err := net.Dial("unix", c.socketPath)
	if err != nil {
		return nil, fmt.Errorf("Failed to dial: %w", err)
	}
	defer conn.Close()

	collateReq := taggedRequest{
		Tag:   "Collate",
		Value: req,
	}

	enc := msgpack.NewEncoder(conn)
	enc.UseArrayEncodedStructs(false)
	buf, err := msgpack.Marshal(collateReq)
	if err != nil {
		return nil, fmt.Errorf("Failed to encode request: %w", err)
	}
	lengthBytes, err := encodeUint32BufLength(buf)
	if err != nil {
		return nil, fmt.Errorf("Failed to encode request length: %w", err)
	}
	_, err = conn.Write(lengthBytes)
	if err != nil {
		return nil, fmt.Errorf("Failed to send request length: %w", err)
	}
	_, err = conn.Write(buf)
	if err != nil {
		return nil, fmt.Errorf("Failed to send request: %w", err)
	}

	var resp *taggedResponse
	dec := msgpack.NewDecoder(conn)
	if err := dec.Decode(&resp); err != nil {
		return nil, fmt.Errorf("Failed to decode response: %w", err)
	}

	switch resp.Tag {
	case "Ok":
		var collateResp []string
		err := msgpack.Unmarshal(resp.Value, &collateResp)
		if err != nil {
			return nil, fmt.Errorf("Could not unmarshal response value: %w", err)
		}
		return collateResp, nil
	case "InternalServerError":
		return nil, fmt.Errorf("Custard had internal server error")
	default:
		return nil, fmt.Errorf("Unrecognised tag from server: %s", resp.Tag)
	}
}
