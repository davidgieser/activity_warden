PROJECT_DIR := ${HOME}/Coding/activity_warden/
BUILD_FLAGS := -r

all: user_daemon firefox_native_messenger gnome_desktop

run_user_daemon:
	@echo Executing $@
	@cd ${PROJECT_DIR}/user_daemon; cargo run ${BUILD_FLAGS}

user_daemon:
	@echo Building $@
	@cd ${PROJECT_DIR}/user_daemon; cargo build ${BUILD_FLAGS}

firefox_native_messenger:
	@echo Building $@
	@cd ${PROJECT_DIR}/firefox_native_messenger; cargo build ${BUILD_FLAGS}

gnome_desktop:
	@echo Building $@
	@cd ${PROJECT_DIR}/gnome_desktop; cargo build ${BUILD_FLAGS}

.PHONY: user_daemon firefox_native_messenger gnome_desktop