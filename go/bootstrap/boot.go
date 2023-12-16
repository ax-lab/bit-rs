package bootstrap

import (
	"fmt"
	"os"
	"path/filepath"
	"syscall"
)

// Performs the boot routine for the bootstrap process. This will check if
// the bootstrapper needs to be update and rebuild itself, restarting the
// process if that is the case.
func Boot(rootDir, rootFile string) {
	exeFile := getBootstrapExe()
	if exeFile == "" {
		return
	}

	rootStat := logBoot(os.Stat(rootDir))
	if rootStat == nil {
		return
	}

	exeStat := logBoot(os.Stat(exeFile))
	if exeStat == nil {
		return
	}

	exeTime := exeStat.ModTime()
	isNewer := rootStat.ModTime().After(exeTime)
	if !isNewer {
		filepath.WalkDir(rootDir, func(path string, dir os.DirEntry, err error) error {
			if isNewer {
				return filepath.SkipDir
			} else if err != nil {
				return err
			} else if info, err := dir.Info(); err == nil {
				isNewer = isNewer || info.ModTime().After(exeTime)
			} else {
				return err
			}
			return nil
		})
	}

	if !isNewer {
		return
	}

	fmt.Printf("\n[bootstrap] rebuilding...\n")
	mainFile := filepath.Join(rootDir, rootFile)
	if !Run("[build]", "go", "build", "-o", exeFile, mainFile) {
		logBootErr(fmt.Errorf("build failed"))
		return
	}

	files := make([]*os.File, 3)
	files[syscall.Stdin] = os.Stdin
	files[syscall.Stdout] = os.Stdout
	files[syscall.Stderr] = os.Stderr

	fmt.Printf("[bootstrap] restarting...\n\n")

	if err := Spawn(exeFile, os.Args...); err != nil {
		logBootErr(fmt.Errorf("restart failed: %v", err))
		os.Exit(123)
	}
}

func getBootstrapExe() string {
	exeFile := logBoot(os.Executable())

	if exeFile != "" {
		exeFile = logBoot(filepath.EvalSymlinks(exeFile))
	}

	if exeFile != "" {
		exeFile = filepath.Clean(exeFile)
	}

	// check if we are running the bootstrap executable (e.g. not `go run`)
	if filepath.Dir(exeFile) != ProjectDir() {
		return ""
	}

	return exeFile
}

func logBoot[T any](val T, err error) T {
	if err != nil {
		location := Caller(1)
		logBootErr(fmt.Errorf("%s%v", location, err))
	}
	return val
}

func logBootErr(err any) {
	fmt.Fprintf(os.Stderr, "[bootstrap] error: %v\n", err)
}
