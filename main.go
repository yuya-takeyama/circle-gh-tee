package main

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"regexp"
	"strconv"
	"strings"
	"text/template"

	"github.com/google/go-github/github"
	flags "github.com/jessevdk/go-flags"
	"github.com/yuya-takeyama/posixexec"
	"golang.org/x/oauth2"
)

const AppName = "circle-gh-tee"

type options struct {
	ExitZeroTemplate    string `long:"exit-zero-template" description:"Comment template used when exit code is zero"`
	ExitNonZeroTemplate string `long:"exit-non-zero-template" description:"Comment template used when exit code is non-zero"`
	ShowVersion         bool   `short:"v" long:"version" description:"Show version"`
}

var opts options

const defaultExitZeroTemplate = ":white_check_mark: `{{.FullCmd}}` exited with `{{.ExitStatus}}`.\n\n" +
	"```\n" +
	"{{.Result}}\n" +
	"```"

const defaultExitNonZeroTemplate = ":no_entry_sign: `{{.FullCmd}}` exited with `{{.ExitStatus}}`.\n" +
	"```\n" +
	"{{.Result}}\n" +
	"```"

type Context struct {
	Cmd        string
	Args       []string
	ExitStatus int
	Result     string
}

func (c *Context) FullCmd() string {
	return c.Cmd + " " + strings.Join(c.Args, " ")
}

func main() {
	parser := flags.NewParser(&opts, flags.Default^flags.PrintErrors)
	parser.Name = AppName
	parser.Usage = "[OPTIONS] -- COMMAND"

	args, err := parser.Parse()

	if err != nil {
		fmt.Fprint(os.Stderr, err)
		return
	}

	if opts.ShowVersion {
		io.WriteString(os.Stdout, fmt.Sprintf("%s v%s, build %s\n", AppName, Version, GitCommit))
		return
	}

	cmdName := args[0]
	cmdArgs := args[1:]

	circleGhTee(cmdName, cmdArgs, os.Stdin, os.Stdout, os.Stderr)
}

func circleGhTee(cmdName string, cmdArgs []string, stdin io.Reader, stdout io.Writer, stderr io.Writer) {
	cmd := exec.Command(cmdName, cmdArgs...)

	resultBuffer := new(bytes.Buffer)
	commentBuffer := new(bytes.Buffer)

	cmd.Stdin = stdin
	cmd.Stdout = io.MultiWriter(stdout, resultBuffer)
	cmd.Stderr = io.MultiWriter(stderr, resultBuffer)

	exitStatus, err := posixexec.Run(cmd)

	if err != nil {
		panic(err)
	}

	ctx := &Context{
		Cmd:        cmdName,
		Args:       cmdArgs,
		ExitStatus: exitStatus,
		Result:     removeAnsiColor(resultBuffer.String()),
	}

	var t *template.Template

	if exitStatus == 0 {
		t = template.Must(template.New("exitZero").Parse(defaultExitZeroTemplate))
	} else {
		t = template.Must(template.New("exitNonZero").Parse(defaultExitNonZeroTemplate))
	}

	tmplErr := t.Execute(commentBuffer, ctx)
	if tmplErr != nil {
		panic(tmplErr)
	}

	prNumber, prNumberErr := getPrNumber()
	if prNumberErr != nil {
		panic(prNumberErr)
	}

	postErr := postComment(os.Getenv("CIRCLE_PR_USERNAME"), os.Getenv("CIRCLE_PR_REPONAME"), prNumber, commentBuffer.String(), os.Getenv("GITHUB_API_TOKEN"))
	if postErr != nil {
		panic(postErr)
	}

	os.Exit(exitStatus)
}

var prNumberRegexp *regexp.Regexp = regexp.MustCompile(`/pull/(\d+)$`)

func getPrNumber() (int, error) {
	if os.Getenv("CI_PULL_REQUEST") != "" {
		matches := prNumberRegexp.FindStringSubmatch(os.Getenv("CI_PULL_REQUEST"))

		i, err := strconv.Atoi(matches[1])
		if err != nil {
			return -1, err
		}

		return i, nil
	}

	return -1, errors.New("Not implemented")
}

func postComment(user string, repo string, prNumber int, comment string, token string) error {
	oauth2Token := &oauth2.Token{
		AccessToken: os.Getenv("GITHUB_API_TOKEN"),
	}
	oauthClient := oauth2.NewClient(oauth2.NoContext, oauth2.StaticTokenSource(oauth2Token))
	client := github.NewClient(oauthClient)
	prComment := &github.IssueComment{Body: &comment}
	_, _, err := client.Issues.CreateComment(user, repo, prNumber, prComment)

	return err
}

var ansiColorRegexp *regexp.Regexp = regexp.MustCompile(`\x1b\[[0-9;]*[mK]`)

func removeAnsiColor(str string) string {
	return ansiColorRegexp.ReplaceAllLiteralString(str, "")
}
