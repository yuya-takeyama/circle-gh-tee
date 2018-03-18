# Circle GH Tee

Command to execute a command and post its result to GitHub Pull Request

## Usage

```
$ circle-gh-tee [OPTIONS] -- <COMMAND>...
```

### Options

* `--exit-zero-template <TEMPLATE>`
  * Comment template used when exit code is zero
* `--exit-non-zero-template <TEMPLATE>`
  * Comment template used when exit code is non-zero

### Template variables
  * `{{full_command}}`
    * Executed command
    * e.g. `make test`
  * `{{result}}`
    * Output of the executed command
    * Both of stdout and stderr are merged
  * `{{exit_status}}`
    * Exit status of the executed command
    * e.g `0`