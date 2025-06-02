# Create new greeter instance with configuration block
greeter = HelloWorld.new(name: 'Ruby') { |g| g.configure(timeout: 1000) }
