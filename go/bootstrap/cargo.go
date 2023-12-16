package bootstrap

import (
	"log"
	"path/filepath"
	"runtime"
)

var projectDir = (func() string {
	dir := filepath.Dir(FileName())
	dir = filepath.Join(dir, "..", "..")
	return dir
})()

// Returns the absolute root directory for the project.
func ProjectDir() string {
	return projectDir
}

func TestsDir() string {
	return filepath.Join(ProjectDir(), ScriptTests)
}

// Returns the root path where to run cargo.
func CargoDir() string {
	return filepath.Join(ProjectDir(), CargoWorkspace)
}

// Returns the Go filename of the caller function.
func FileName() string {
	_, callerFile, _, hasInfo := runtime.Caller(1)
	if !hasInfo {
		log.Fatal("could not retrieve caller file name")
	}
	if !filepath.IsAbs(callerFile) {
		log.Fatal("caller file name is not an absolute path")
	}
	return filepath.Clean(callerFile)
}
