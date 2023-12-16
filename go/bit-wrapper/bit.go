package main

import (
	"fmt"
	"os"

	"axlab.dev/bit/bootstrap"
	"axlab.dev/bit/util"
)

func main() {
	bootstrap.Boot("go", "bit-wrapper/bit.go")

	var cargoDir = bootstrap.CargoDir()
	var (
		clean   = os.Getenv("CLEAN") != ""
		release = os.Getenv("RELEASE") != ""
	)

	if clean {
		fmt.Printf("\nCleaning up source files...\n\n")
	}

	var success = bootstrap.ExecInDir("[cargo]", cargoDir, func() bool {
		if clean {
			return bootstrap.Run("[cargo]", "cargo", "clean")
		}
		args := []string{"build", "--quiet"}
		if release {
			args = append(args, "--release")
		}

		return bootstrap.Run("[cargo]", "cargo", args...)
	})

	if clean {
		return
	}

	if success {
		var (
			args = os.Args
			verb = ""
		)

		var verbArgs []string
		if len(args) > 1 {
			verb, verbArgs = args[1], args[2:]
		}

		var (
			failure = 0
			success = 0
			skipped = 0

			all []bootstrap.ScriptTest
		)

		var bit = bootstrap.NewRunner(release)
		switch verb {
		case "test":
			tests := bootstrap.TestsDir()
			files := util.Glob(tests, "*.bit")
			for _, it := range files {
				matches := len(verbArgs) == 0
				for _, pattern := range verbArgs {
					matches = matches || util.MatchesPattern(it, pattern)
				}

				if !matches {
					continue
				}

				out := bootstrap.RunScriptTest(it)
				all = append(all, out)
				if out.Skipped {
					skipped++
				} else if out.Success {
					success++
				} else {
					failure++
				}
			}

			summary := func() {
				fmt.Printf("\n=== [ SUMMARY - Tests: %d", len(all))
				if failure > 0 {
					fmt.Printf(" / Failed: %d", failure)
				} else {
					fmt.Printf(" / Passed: %d", success)
				}
				if skipped > 0 {
					fmt.Printf(" / Skipped: %d", skipped)
				}
				fmt.Printf(" ]\n\n")
			}

			for _, test := range all {
				test.OutputDetails()
			}

			summary()

			if failure > 0 {
				fmt.Printf("    ## Failed tests ##\n\n")
				for _, test := range all {
					if !test.Skipped && !test.Success {
						fmt.Printf("    - %s\n", test.Name)
					}
				}
				fmt.Printf("\n")
				os.Exit(1)
			}

		default:
			bit.Spawn(args...)
		}
	}
}
