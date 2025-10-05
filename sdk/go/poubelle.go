package poubelle

import (
	"bufio"
	"encoding/json"
	"fmt"
	"net"
	"regexp"
	"strconv"
	"strings"
)

type Client struct {
	conn     net.Conn
	host     string
	port     int
	username string
	password string
}

type Row map[string]interface{}

func NewClient(connectionString string) (*Client, error) {
	host, port, username, password, err := parseConnectionString(connectionString)
	if err != nil {
		return nil, err
	}

	return &Client{
		host:     host,
		port:     port,
		username: username,
		password: password,
	}, nil
}

func parseConnectionString(connStr string) (string, int, string, string, error) {
	re := regexp.MustCompile(`^poubelle://([^:]+):([^@]+)@([^:]+):(\d+)$`)
	matches := re.FindStringSubmatch(connStr)

	if matches == nil {
		return "", 0, "", "", fmt.Errorf("invalid connection string format. Expected: poubelle://username:password@host:port")
	}

	port, err := strconv.Atoi(matches[4])
	if err != nil {
		return "", 0, "", "", fmt.Errorf("invalid port number: %v", err)
	}

	return matches[3], port, matches[1], matches[2], nil
}

func (c *Client) Connect() error {
	addr := fmt.Sprintf("%s:%d", c.host, c.port)
	conn, err := net.Dial("tcp", addr)
	if err != nil {
		return fmt.Errorf("connection failed: %v", err)
	}

	c.conn = conn
	reader := bufio.NewReader(conn)

	if err := waitForPrompt(reader, "Username: "); err != nil {
		return err
	}
	if _, err := fmt.Fprintf(conn, "%s\n", c.username); err != nil {
		return err
	}

	if err := waitForPrompt(reader, "Password: "); err != nil {
		return err
	}
	if _, err := fmt.Fprintf(conn, "%s\n", c.password); err != nil {
		return err
	}

	if err := waitForPrompt(reader, "Connected to Poubelle DB"); err != nil {
		return fmt.Errorf("authentication failed")
	}

	return nil
}

func (c *Client) Query(sql string) (string, error) {
	if c.conn == nil {
		return "", fmt.Errorf("not connected")
	}

	reader := bufio.NewReader(c.conn)

	if err := waitForPrompt(reader, "poubelle> "); err != nil {
		return "", err
	}

	if _, err := fmt.Fprintf(c.conn, "%s\n", sql); err != nil {
		return "", err
	}

	result, err := readUntilPrompt(reader, "poubelle> ")
	if err != nil {
		return "", err
	}

	return strings.TrimSpace(result), nil
}

func (c *Client) Execute(sql string) ([]Row, error) {
	result, err := c.Query(sql)
	if err != nil {
		return nil, err
	}

	return parseRows(result), nil
}

func (c *Client) ExecuteJSON(sql string) ([]Row, error) {
	if !strings.Contains(strings.ToUpper(sql), "FORMAT JSON") {
		sql = sql + " FORMAT JSON"
	}

	result, err := c.Query(sql)
	if err != nil {
		return nil, err
	}

	var rows []Row
	if err := json.Unmarshal([]byte(result), &rows); err != nil {
		return nil, fmt.Errorf("failed to parse JSON: %v", err)
	}

	return rows, nil
}

func (c *Client) Close() error {
	if c.conn != nil {
		fmt.Fprintf(c.conn, "exit\n")
		return c.conn.Close()
	}
	return nil
}

func waitForPrompt(reader *bufio.Reader, prompt string) error {
	buffer := ""
	for {
		b, err := reader.ReadByte()
		if err != nil {
			return err
		}
		buffer += string(b)
		if strings.Contains(buffer, prompt) {
			break
		}
	}
	return nil
}

func readUntilPrompt(reader *bufio.Reader, prompt string) (string, error) {
	buffer := ""
	for {
		b, err := reader.ReadByte()
		if err != nil {
			return "", err
		}
		buffer += string(b)
		if strings.Contains(buffer, prompt) {
			result := strings.TrimSuffix(buffer, prompt)
			return strings.TrimSpace(result), nil
		}
	}
}

func parseRows(result string) []Row {
	if result == "" || result == "No rows" {
		return []Row{}
	}

	if !strings.Contains(result, "{") {
		return []Row{}
	}

	var rows []Row
	lines := strings.Split(result, "\n")

	for _, line := range lines {
		line = strings.TrimSpace(line)
		if row := parseRow(line); row != nil {
			rows = append(rows, row)
		}
	}

	return rows
}

func parseRow(line string) Row {
	if !strings.HasPrefix(line, "{") || !strings.HasSuffix(line, "}") {
		return nil
	}

	inner := line[1 : len(line)-1]
	row := make(Row)

	parts := strings.Split(inner, ", ")
	for _, part := range parts {
		kv := strings.SplitN(part, ": ", 2)
		if len(kv) != 2 {
			continue
		}

		key := strings.Trim(kv[0], "\"")
		value := parseValue(kv[1])
		row[key] = value
	}

	if len(row) == 0 {
		return nil
	}

	return row
}

func parseValue(value string) interface{} {
	value = strings.TrimSpace(value)

	if value == "Null" {
		return nil
	}

	if strings.HasPrefix(value, "Int(") && strings.HasSuffix(value, ")") {
		numStr := value[4 : len(value)-1]
		if num, err := strconv.ParseInt(numStr, 10, 64); err == nil {
			return num
		}
	}

	if strings.HasPrefix(value, "Text(") && strings.HasSuffix(value, ")") {
		text := value[5 : len(value)-1]
		return strings.Trim(text, "\"")
	}

	return value
}
